package conf_test

import (
	"encoding/json"
	"testing"

	. "github.com/xtls/xray-core/infra/conf"
)

// GeekVPN: allow-plain-vless.patch must let a plain VLESS outbound through
// (a tunnel config: ws, security none, a public domain), while plain Trojan
// is still refused.
func TestGeekVPNPlainVless(t *testing.T) {
	build := func(raw string) error {
		var c OutboundDetourConfig
		if err := json.Unmarshal([]byte(raw), &c); err != nil {
			t.Fatal(err)
		}
		_, err := c.Build()
		return err
	}

	vless := `{"protocol":"vless","tag":"proxy",
		"settings":{"vnext":[{"address":"cdn.example.com","port":2086,
			"users":[{"id":"27848739-7e62-4138-9fd3-098a63964b6b","encryption":"none"}]}]},
		"streamSettings":{"network":"ws","security":"none","wsSettings":{"path":"/","host":"origin.example.com"}}}`
	if err := build(vless); err != nil {
		t.Fatalf("plain VLESS refused: %v", err)
	}

	trojan := `{"protocol":"trojan","tag":"proxy",
		"settings":{"servers":[{"address":"cdn.example.com","port":2086,"password":"x"}]},
		"streamSettings":{"network":"tcp","security":"none"}}`
	if err := build(trojan); err == nil {
		t.Fatal("plain Trojan accepted; the patch should touch VLESS only")
	}
}
