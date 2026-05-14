//go:build cabi

// HTTP serve helper isolated so the export functions stay readable.

package main

import "net/http"

func httpServe(addr string, h http.Handler) error {
	return http.ListenAndServe(addr, h)
}
