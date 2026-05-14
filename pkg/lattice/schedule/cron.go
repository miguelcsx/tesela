// Package schedule is a small cron-like scheduler. It implements a
// 5-field cron expression parser (minute hour day-of-month month day-of-week)
// and a scheduler that runs registered jobs.
//
// Intentionally dependency-free; for very advanced needs (timezones,
// distributed locks) plug in an external runtime via Schedule.External.

package schedule

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Spec is a parsed 5-field cron expression. Each field is a set of
// allowed integer values; a nil set means "any".
type Spec struct {
	Minute  set
	Hour    set
	Day     set // 1-31
	Month   set // 1-12
	Weekday set // 0-6 (0 = Sunday)
}

// set is the field representation. nil = wildcard; otherwise membership.
type set map[int]struct{}

// Parse a 5-field cron string. Supports: *, comma lists ("1,2,3"), ranges
// ("1-5"), step ("*/5", "0-30/5"). Day names and month names are not
// supported.
func Parse(expr string) (Spec, error) {
	parts := strings.Fields(expr)
	if len(parts) != 5 {
		return Spec{}, fmt.Errorf("schedule: cron expr must have 5 fields, got %d in %q", len(parts), expr)
	}
	min, err := parseField(parts[0], 0, 59)
	if err != nil {
		return Spec{}, fmt.Errorf("minute: %w", err)
	}
	hr, err := parseField(parts[1], 0, 23)
	if err != nil {
		return Spec{}, fmt.Errorf("hour: %w", err)
	}
	day, err := parseField(parts[2], 1, 31)
	if err != nil {
		return Spec{}, fmt.Errorf("day: %w", err)
	}
	mo, err := parseField(parts[3], 1, 12)
	if err != nil {
		return Spec{}, fmt.Errorf("month: %w", err)
	}
	wd, err := parseField(parts[4], 0, 6)
	if err != nil {
		return Spec{}, fmt.Errorf("weekday: %w", err)
	}
	return Spec{Minute: min, Hour: hr, Day: day, Month: mo, Weekday: wd}, nil
}

// Match reports whether t falls into this Spec's schedule.
func (s Spec) Match(t time.Time) bool {
	return inSet(s.Minute, t.Minute()) &&
		inSet(s.Hour, t.Hour()) &&
		inSet(s.Day, t.Day()) &&
		inSet(s.Month, int(t.Month())) &&
		inSet(s.Weekday, int(t.Weekday()))
}

// Next returns the next minute boundary at or after `from` that matches.
// Returns ok=false if no match is found within 4 years (i.e., expr matches
// nothing — typically a bug in the user expr).
func (s Spec) Next(from time.Time) (time.Time, bool) {
	t := from.Truncate(time.Minute).Add(time.Minute)
	end := t.AddDate(4, 0, 0)
	for !t.After(end) {
		if s.Match(t) {
			return t, true
		}
		t = t.Add(time.Minute)
	}
	return time.Time{}, false
}

func inSet(s set, v int) bool {
	if s == nil {
		return true
	}
	_, ok := s[v]
	return ok
}

func parseField(field string, lo, hi int) (set, error) {
	if field == "*" {
		return nil, nil
	}
	out := set{}
	for _, sub := range strings.Split(field, ",") {
		step := 1
		base := sub
		if i := strings.Index(sub, "/"); i >= 0 {
			s, err := strconv.Atoi(sub[i+1:])
			if err != nil || s <= 0 {
				return nil, fmt.Errorf("bad step %q", sub)
			}
			step = s
			base = sub[:i]
		}
		var a, b int
		var err error
		switch {
		case base == "*":
			a, b = lo, hi
		case strings.Contains(base, "-"):
			parts := strings.SplitN(base, "-", 2)
			a, err = strconv.Atoi(parts[0])
			if err != nil {
				return nil, fmt.Errorf("bad range start %q", base)
			}
			b, err = strconv.Atoi(parts[1])
			if err != nil {
				return nil, fmt.Errorf("bad range end %q", base)
			}
		default:
			a, err = strconv.Atoi(base)
			if err != nil {
				return nil, fmt.Errorf("bad value %q", base)
			}
			b = a
		}
		if a < lo || b > hi || a > b {
			return nil, fmt.Errorf("range %d-%d out of bounds [%d,%d]", a, b, lo, hi)
		}
		for v := a; v <= b; v += step {
			out[v] = struct{}{}
		}
	}
	return out, nil
}

// Job is a scheduled function. Returning an error logs the failure but
// does not stop the scheduler.
type Job func(ctx context.Context) error

// Scheduler runs registered jobs at their scheduled times. Single-process;
// for distributed locking, wrap a Job with your own lock/lease.
type Scheduler struct {
	mu   sync.Mutex
	jobs []scheduledJob
	stop chan struct{}
	now  func() time.Time
}

type scheduledJob struct {
	id   string
	spec Spec
	fn   Job
	last time.Time
}

// NewScheduler returns a stopped Scheduler.
func NewScheduler() *Scheduler {
	return &Scheduler{stop: make(chan struct{}), now: func() time.Time { return time.Now().UTC() }}
}

// Add registers a job at the given cron expression. Returns the job id.
func (s *Scheduler) Add(id, expr string, fn Job) error {
	if fn == nil {
		return errors.New("schedule: nil job")
	}
	spec, err := Parse(expr)
	if err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.jobs = append(s.jobs, scheduledJob{id: id, spec: spec, fn: fn})
	return nil
}

// Remove cancels a job by id.
func (s *Scheduler) Remove(id string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := s.jobs[:0]
	for _, j := range s.jobs {
		if j.id != id {
			out = append(out, j)
		}
	}
	s.jobs = out
}

// Run blocks until ctx is cancelled, ticking every minute and dispatching
// any matching jobs. Errors from jobs are swallowed by design (cron jobs
// must be self-contained); use a logger inside the job to surface them.
func (s *Scheduler) Run(ctx context.Context) error {
	tick := time.NewTicker(time.Minute)
	defer tick.Stop()
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-s.stop:
			return nil
		case t := <-tick.C:
			s.tick(ctx, t.UTC())
		}
	}
}

func (s *Scheduler) tick(ctx context.Context, t time.Time) {
	s.mu.Lock()
	jobs := append([]scheduledJob(nil), s.jobs...)
	s.mu.Unlock()
	for i := range jobs {
		if !jobs[i].spec.Match(t) {
			continue
		}
		// Skip if we already ran this minute.
		if !jobs[i].last.IsZero() && jobs[i].last.Truncate(time.Minute).Equal(t.Truncate(time.Minute)) {
			continue
		}
		jobs[i].last = t
		go func(j scheduledJob) { _ = j.fn(ctx) }(jobs[i])
	}
	// Persist updated last-run timestamps.
	s.mu.Lock()
	s.jobs = jobs
	s.mu.Unlock()
}

// Stop signals Run to exit on the next loop iteration.
func (s *Scheduler) Stop() {
	select {
	case <-s.stop:
	default:
		close(s.stop)
	}
}
