package schedule_test

import (
	"testing"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/schedule"
)

func TestParse_AllFields(t *testing.T) {
	cases := []struct {
		expr  string
		match string
		want  bool
	}{
		{"* * * * *", "2026-05-07T12:00:00Z", true},
		{"0 12 * * *", "2026-05-07T12:00:00Z", true},
		{"0 12 * * *", "2026-05-07T13:00:00Z", false},
		{"*/5 * * * *", "2026-05-07T12:05:00Z", true},
		{"*/5 * * * *", "2026-05-07T12:06:00Z", false},
		{"0 9-17 * * 1-5", "2026-05-08T10:00:00Z", true},  // Friday
		{"0 9-17 * * 1-5", "2026-05-09T10:00:00Z", false}, // Saturday
	}
	for _, c := range cases {
		s, err := schedule.Parse(c.expr)
		if err != nil {
			t.Fatalf("parse %q: %v", c.expr, err)
		}
		ts, _ := time.Parse(time.RFC3339, c.match)
		if got := s.Match(ts); got != c.want {
			t.Errorf("Match(%q, %q) = %v, want %v", c.expr, c.match, got, c.want)
		}
	}
}

func TestParse_Errors(t *testing.T) {
	bad := []string{
		"* * * *",     // 4 fields
		"60 * * * *",  // out of range
		"* * 0 * *",   // out of range
		"* * * * 7",   // out of range
		"foo * * * *", // bad value
	}
	for _, b := range bad {
		if _, err := schedule.Parse(b); err == nil {
			t.Errorf("expected error for %q", b)
		}
	}
}

func TestNext(t *testing.T) {
	s, _ := schedule.Parse("0 9 * * 1-5")
	from, _ := time.Parse(time.RFC3339, "2026-05-09T08:00:00Z") // Saturday
	next, ok := s.Next(from)
	if !ok {
		t.Fatal("no next match")
	}
	// Next 9:00 weekday after Saturday 8am = Monday 9am
	if next.Weekday() != time.Monday || next.Hour() != 9 {
		t.Fatalf("expected Monday 9:00, got %s", next)
	}
}
