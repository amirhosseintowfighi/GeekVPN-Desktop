package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/amirhosseintowfighi/geekvpn-android/cfscan"
	corenet "github.com/xtls/xray-core/common/net"
	core "github.com/xtls/xray-core/core"
	corestats "github.com/xtls/xray-core/features/stats"
	"github.com/xtls/xray-core/infra/conf/serial"

	// Registers every protocol, transport and app Xray ships, as the
	// official binary does.
	_ "github.com/xtls/xray-core/main/distro/all"
)

// The same default as libv2ray, so delays read the same on both clients.
const defaultDelayURL = "https://www.gstatic.com/generate_204"

type engine struct {
	out *out

	mu      sync.Mutex
	running *core.Instance

	scanner *cfscan.Scanner
}

func newEngine(o *out) *engine {
	return &engine{out: o, scanner: cfscan.NewScanner()}
}

func (e *engine) handle(method string, params json.RawMessage) (any, error) {
	switch method {
	case "version":
		return map[string]string{"geekcore": version, "xray": core.Version(), "cfscan": cfscan.Version()}, nil
	case "core.start":
		p, err := decode[struct {
			Config json.RawMessage `json:"config"`
		}](params)
		if err != nil {
			return nil, err
		}
		return nil, e.start(p.Config)
	case "core.stop":
		e.stop()
		return nil, nil
	case "core.delay":
		p, err := decode[struct {
			URL string `json:"url"`
		}](params)
		if err != nil {
			return nil, err
		}
		inst := e.instance()
		if inst == nil {
			return nil, errors.New("core is not running")
		}
		ctx, cancel := context.WithTimeout(context.Background(), 12*time.Second)
		defer cancel()
		ms, err := measure(ctx, inst, p.URL)
		if err != nil {
			return nil, err
		}
		return map[string]int64{"ms": ms}, nil
	case "core.traffic":
		return e.traffic(), nil
	case "test.delay":
		p, err := decode[delayTest](params)
		if err != nil {
			return nil, err
		}
		return e.testDelays(p), nil
	case "scan.start":
		p, err := decode[struct {
			Config json.RawMessage `json:"config"`
			Ranges string          `json:"ranges"`
		}](params)
		if err != nil {
			return nil, err
		}
		return nil, e.scanner.Start(string(p.Config), p.Ranges, scanListener{e.out})
	case "scan.stop":
		e.scanner.Stop()
		return nil, nil
	}
	return nil, fmt.Errorf("unknown method %q", method)
}

// -- the tunnel ----------------------------------------------------------------

func load(config json.RawMessage) (*core.Instance, error) {
	c, err := serial.LoadJSONConfig(strings.NewReader(string(config)))
	if err != nil {
		return nil, fmt.Errorf("config: %w", err)
	}
	inst, err := core.New(c)
	if err != nil {
		return nil, fmt.Errorf("core: %w", err)
	}
	return inst, nil
}

// start replaces whatever runs with this config. The old instance is closed
// first: both would bind the same local proxy ports.
func (e *engine) start(config json.RawMessage) error {
	e.stop()
	inst, err := load(config)
	if err != nil {
		return err
	}
	if err := inst.Start(); err != nil {
		_ = inst.Close()
		return fmt.Errorf("start: %w", err)
	}
	e.mu.Lock()
	e.running = inst
	e.mu.Unlock()
	return nil
}

func (e *engine) stop() {
	e.mu.Lock()
	inst := e.running
	e.running = nil
	e.mu.Unlock()
	if inst != nil {
		_ = inst.Close()
	}
}

func (e *engine) instance() *core.Instance {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.running
}

func (e *engine) shutdown() {
	e.scanner.Stop()
	e.stop()
}

// traffic reads the proxy outbound's byte counters: the config asks Xray to
// keep them (policy.system.statsOutbound*). Cumulative since start; the app
// turns two readings into a speed.
func (e *engine) traffic() map[string]int64 {
	res := map[string]int64{"up": 0, "down": 0}
	inst := e.instance()
	if inst == nil {
		return res
	}
	m, ok := inst.GetFeature(corestats.ManagerType()).(corestats.Manager)
	if !ok || m == nil {
		return res
	}
	for key, name := range map[string]string{"up": "uplink", "down": "downlink"} {
		if c := m.GetCounter("outbound>>>proxy>>>traffic>>>" + name); c != nil {
			res[key] = c.Value()
		}
	}
	return res
}

// -- delay tests -----------------------------------------------------------------

type delayTest struct {
	Items []struct {
		ID     string          `json:"id"`
		Config json.RawMessage `json:"config"`
	} `json:"items"`
	URL         string `json:"url"`
	Concurrency int    `json:"concurrency"`
}

type delayResult struct {
	ID    string `json:"id"`
	Ms    int64  `json:"ms"`
	Error string `json:"error,omitempty"`
}

// testDelays is v2rayNG's real-delay test: each config gets its own
// in-process Xray instance with no inbounds, and an HTTP request dialled
// through it. Results stream as test.result events and come back together.
func (e *engine) testDelays(t delayTest) []delayResult {
	n := t.Concurrency
	if n <= 0 {
		n = 8
	}
	if n > len(t.Items) {
		n = len(t.Items)
	}
	results := make([]delayResult, len(t.Items))
	var mu sync.Mutex
	sem := make(chan struct{}, n)
	var wg sync.WaitGroup
	for i, item := range t.Items {
		wg.Add(1)
		sem <- struct{}{}
		go func(i int, id string, config json.RawMessage) {
			defer wg.Done()
			defer func() { <-sem }()
			r := delayResult{ID: id, Ms: -1}
			if ms, err := delayOf(config, t.URL); err != nil {
				r.Error = err.Error()
			} else {
				r.Ms = ms
			}
			mu.Lock()
			results[i] = r
			mu.Unlock()
			e.out.emit("test.result", r)
		}(i, item.ID, item.Config)
	}
	wg.Wait()
	return results
}

func delayOf(config json.RawMessage, url string) (int64, error) {
	inst, err := load(config)
	if err != nil {
		return -1, err
	}
	if err := inst.Start(); err != nil {
		return -1, err
	}
	defer inst.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 12*time.Second)
	defer cancel()
	return measure(ctx, inst, url)
}

// measure: two GETs through the instance, the faster one counts - the first
// pays for the handshake. Same method as libv2ray's measureInstDelay.
func measure(ctx context.Context, inst *core.Instance, url string) (int64, error) {
	if url == "" {
		url = defaultDelayURL
	}
	tr := &http.Transport{
		TLSHandshakeTimeout: 6 * time.Second,
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			dest, err := corenet.ParseDestination(network + ":" + addr)
			if err != nil {
				return nil, err
			}
			return core.Dial(ctx, inst, dest)
		},
	}
	defer tr.CloseIdleConnections()
	client := &http.Client{Transport: tr}

	best := int64(-1)
	var lastErr error
	for i := 0; i < 2; i++ {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
		if err != nil {
			return -1, err
		}
		start := time.Now()
		resp, err := client.Do(req)
		if err != nil {
			lastErr = err
			continue
		}
		_, _ = io.Copy(io.Discard, resp.Body)
		resp.Body.Close()
		if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusNoContent {
			lastErr = fmt.Errorf("status %s", resp.Status)
			continue
		}
		if ms := time.Since(start).Milliseconds(); best < 0 || ms < best {
			best = ms
		}
	}
	if best < 0 {
		if lastErr == nil {
			lastErr = errors.New("no answer")
		}
		return -1, lastErr
	}
	return best, nil
}

// -- the scanner -------------------------------------------------------------------

type scanListener struct{ out *out }

func (l scanListener) OnResult(resultJSON string) {
	l.out.emit("scan.result", json.RawMessage(resultJSON))
}

func (l scanListener) OnProgress(tested, total, found int64) {
	l.out.emit("scan.progress", map[string]int64{"tested": tested, "total": total, "found": found})
}

func (l scanListener) OnFinish(errorMessage string) {
	l.out.emit("scan.finish", map[string]string{"error": errorMessage})
}
