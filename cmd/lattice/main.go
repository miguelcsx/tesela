// Command lattice is the operator CLI. It composes cobra subcommands; each
// subcommand lives in its own file and is registered in newRootCmd.

package main

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"

	"github.com/miguelcsx/lattice/pkg/lattice/buildinfo"
)

func main() {
	if err := newRootCmd().Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func newRootCmd() *cobra.Command {
	root := &cobra.Command{
		Use:           "lattice",
		Short:         "Lattice — ontology-driven application runtime",
		Long:          "Operator CLI for Lattice. Apply ontologies, query objects, manage migrations, and inspect audit logs.",
		Version:       buildinfo.Version,
		SilenceUsage:  true,
		SilenceErrors: true,
	}
	root.PersistentFlags().String("config", "", "path to YAML config file")
	root.PersistentFlags().String("server", "", "lattice-api base URL (overrides config)")
	root.PersistentFlags().String("token", "", "bearer token (overrides LATTICE_TOKEN)")
	root.PersistentFlags().String("workspace", "", "workspace api_name (required for most subcommands)")

	root.AddCommand(newOntologyCmd())
	root.AddCommand(newQueryCmd())
	root.AddCommand(newDBCmd())
	root.AddCommand(newAuditCmd())
	return root
}
