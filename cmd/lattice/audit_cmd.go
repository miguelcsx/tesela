// `lattice audit tail` subcommand — fetch the most recent audit records.

package main

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
)

func newAuditCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "audit", Short: "Inspect the audit log"}
	cmd.AddCommand(newAuditTailCmd())
	return cmd
}

func newAuditTailCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "tail",
		Short: "Fetch recent audit records for the workspace",
		RunE: func(cmd *cobra.Command, _ []string) error {
			c, err := newClient(cmd)
			if err != nil {
				return err
			}
			if c.workspace == "" {
				return fmt.Errorf("--workspace is required")
			}
			out, err := c.get(context.Background(),
				fmt.Sprintf("/v1/workspaces/%s/audit", c.workspace))
			if err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), string(out))
			return nil
		},
	}
}
