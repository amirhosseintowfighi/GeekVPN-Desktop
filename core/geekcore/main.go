// Command geekcore is GeekVPN desktop's connection engine: the Xray core and
// the Cloudflare clean-IP scanner (cfscan) in one process, driven by the app
// over stdin/stdout.
//
// It is the desktop counterpart of the Android app's libv2ray.aar, built from
// the same Xray commit with the same patches and the same cfscan module, so a
// config that connects on one client connects on the other.
//
// Protocol: one JSON object per line. Requests carry an id and a method;
// every request gets exactly one response with the same id. Unsolicited
// events (scanner progress, delay-test results) have no id. Xray's own log
// goes to stderr, which the app keeps for problem reports.
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"sync"
)

// version identifies this engine in the About screen and problem reports.
const version = "0.1.0"

type request struct {
	ID     int64           `json:"id"`
	Method string          `json:"method"`
	Params json.RawMessage `json:"params"`
}

type response struct {
	ID     int64  `json:"id"`
	Result any    `json:"result,omitempty"`
	Error  string `json:"error,omitempty"`
}

type event struct {
	Event string `json:"event"`
	Data  any    `json:"data"`
}

// out serialises every line written to stdout: responses and events come
// from many goroutines and must never interleave mid-line.
type out struct {
	mu  sync.Mutex
	enc *json.Encoder
}

func (o *out) send(v any) {
	o.mu.Lock()
	defer o.mu.Unlock()
	_ = o.enc.Encode(v)
}

func (o *out) emit(name string, data any) { o.send(event{Event: name, Data: data}) }

func main() {
	o := &out{enc: json.NewEncoder(os.Stdout)}
	e := newEngine(o)

	in := bufio.NewScanner(os.Stdin)
	// A full Xray config for many servers can be large.
	in.Buffer(make([]byte, 1<<20), 64<<20)
	var pending sync.WaitGroup
	for in.Scan() {
		var req request
		if err := json.Unmarshal(in.Bytes(), &req); err != nil {
			o.send(response{ID: 0, Error: "bad request: " + err.Error()})
			continue
		}
		// Each request runs on its own goroutine: a delay test of fifty
		// servers must not hold up a disconnect.
		pending.Add(1)
		go func(req request) {
			defer pending.Done()
			result, err := e.handle(req.Method, req.Params)
			if err != nil {
				o.send(response{ID: req.ID, Error: err.Error()})
				return
			}
			if result == nil {
				result = struct{}{}
			}
			o.send(response{ID: req.ID, Result: result})
		}(req)
	}
	// stdin closed: the app is gone, so is the tunnel it asked for. Answer
	// what was already asked first; a supervisor that closes stdin after its
	// last request still gets its reply.
	pending.Wait()
	e.shutdown()
}

func decode[T any](raw json.RawMessage) (T, error) {
	var v T
	if len(raw) == 0 {
		return v, nil
	}
	if err := json.Unmarshal(raw, &v); err != nil {
		return v, fmt.Errorf("bad params: %w", err)
	}
	return v, nil
}
