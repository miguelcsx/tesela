// `lattice db migrate` subcommands. Operates on the metadata DB directly,
// not via the HTTP API — this is the bootstrap path before lattice-api can
// even start.

package main

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"

	"github.com/miguelcsx/lattice/pkg/lattice/config"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
)

func newDBCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "db", Short: "Metadata database management"}
	cmd.AddCommand(newDBMigrateCmd())
	return cmd
}

func newDBMigrateCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "migrate", Short: "Apply or roll back metadata migrations"}
	cmd.AddCommand(newDBMigrateUpCmd())
	cmd.AddCommand(newDBMigrateDownCmd())
	cmd.AddCommand(newDBMigrateStatusCmd())
	return cmd
}

func newDBMigrateUpCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "up",
		Short: "Apply all pending migrations",
		RunE: func(cmd *cobra.Command, _ []string) error {
			cfg, err := config.LoadCLI(config.LoadOptions{File: stringFlag(cmd, "config")})
			if err != nil {
				return err
			}
			if cfg.MetadataDB.DSN == "" {
				return fmt.Errorf("metadata_db.dsn is required")
			}
			return storage.MigrateUp(context.Background(), cfg.MetadataDB.DSN)
		},
	}
}

func newDBMigrateDownCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "down",
		Short: "Roll back the most recent migration",
		RunE: func(cmd *cobra.Command, _ []string) error {
			cfg, err := config.LoadCLI(config.LoadOptions{File: stringFlag(cmd, "config")})
			if err != nil {
				return err
			}
			return storage.MigrateDown(context.Background(), cfg.MetadataDB.DSN)
		},
	}
}

func newDBMigrateStatusCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "status",
		Short: "Report the current migration version",
		RunE: func(cmd *cobra.Command, _ []string) error {
			cfg, err := config.LoadCLI(config.LoadOptions{File: stringFlag(cmd, "config")})
			if err != nil {
				return err
			}
			v, err := storage.MigrationStatus(context.Background(), cfg.MetadataDB.DSN)
			if err != nil {
				return err
			}
			fmt.Fprintf(cmd.OutOrStdout(), "version: %d\n", v)
			return nil
		},
	}
}
