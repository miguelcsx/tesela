// `lattice ontology` subcommands: apply, export, diff, check.

package main

import (
	"context"
	"fmt"
	"os"

	"github.com/spf13/cobra"

	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
)

func newOntologyCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "ontology", Short: "Apply, export, and diff ontologies"}
	cmd.AddCommand(newOntologyApplyCmd())
	cmd.AddCommand(newOntologyExportCmd())
	cmd.AddCommand(newOntologyDiffCmd())
	cmd.AddCommand(newOntologyCheckCmd())
	return cmd
}

func newOntologyApplyCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "apply <file.json>",
		Short: "Apply an ontology spec to a workspace",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			body, err := os.ReadFile(args[0])
			if err != nil {
				return err
			}
			c, err := newClient(cmd)
			if err != nil {
				return err
			}
			if c.workspace == "" {
				return fmt.Errorf("--workspace is required")
			}
			out, err := c.postRaw(context.Background(),
				fmt.Sprintf("/v1/workspaces/%s/ontology:apply", c.workspace),
				body, "application/json")
			if err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), string(out))
			return nil
		},
	}
	return cmd
}

func newOntologyExportCmd() *cobra.Command {
	var output string
	cmd := &cobra.Command{
		Use:   "export",
		Short: "Export the active ontology as JSON",
		RunE: func(cmd *cobra.Command, _ []string) error {
			c, err := newClient(cmd)
			if err != nil {
				return err
			}
			if c.workspace == "" {
				return fmt.Errorf("--workspace is required")
			}
			out, err := c.get(context.Background(),
				fmt.Sprintf("/v1/workspaces/%s/ontology:export", c.workspace))
			if err != nil {
				return err
			}
			if output != "" {
				return os.WriteFile(output, out, 0o644)
			}
			fmt.Fprintln(cmd.OutOrStdout(), string(out))
			return nil
		},
	}
	cmd.Flags().StringVarP(&output, "output", "o", "", "write to file instead of stdout")
	return cmd
}

func newOntologyDiffCmd() *cobra.Command {
	var from, to string
	cmd := &cobra.Command{
		Use:   "diff",
		Short: "Diff two published ontology versions",
		RunE: func(cmd *cobra.Command, _ []string) error {
			c, err := newClient(cmd)
			if err != nil {
				return err
			}
			if from == "" || to == "" {
				return fmt.Errorf("--from and --to are required")
			}
			out, err := c.get(context.Background(),
				fmt.Sprintf("/v1/workspaces/%s/ontology:diff?from=%s&to=%s", c.workspace, from, to))
			if err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), string(out))
			return nil
		},
	}
	cmd.Flags().StringVar(&from, "from", "", "source version name")
	cmd.Flags().StringVar(&to, "to", "", "target version name")
	return cmd
}

func newOntologyCheckCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "check <file.json>",
		Short: "Validate an ontology spec locally without contacting a server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			body, err := os.ReadFile(args[0])
			if err != nil {
				return err
			}
			doc, err := ontology.ParseDocument(body)
			if err != nil {
				return err
			}
			mat, err := doc.Materialize("")
			if err != nil {
				return err
			}
			if err := ontology.Validate(mat); err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), "ok")
			return nil
		},
	}
}
