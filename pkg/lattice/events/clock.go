package events

import "time"

// nowFn is the time source — overridable in tests.
var nowFn = func() time.Time { return time.Now().UTC() }
