// Embed the SQL migration files into the binary so they ship with every
// build and goose can execute them without any external file lookups.

package migrations

import "embed"

// FS contains every *.sql migration in this directory.
//
//go:embed *.sql
var FS embed.FS
