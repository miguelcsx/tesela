// `lattice query` subcommand — issue a search via the operational API.

package main

import (
	"context"
	"fmt"
	"strings"

	"github.com/spf13/cobra"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func newQueryCmd() *cobra.Command {
	var where string
	var limit int
	cmd := &cobra.Command{
		Use:   "query <ObjectType>",
		Short: "Search objects via the lattice-api",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := newClient(cmd)
			if err != nil {
				return err
			}
			if c.workspace == "" {
				return fmt.Errorf("--workspace is required")
			}
			spec := types.QuerySpec{Page: types.PageSpec{Limit: limit}}
			if where != "" {
				f, err := parseShorthand(where)
				if err != nil {
					return err
				}
				spec.Filter = f
			}
			path := fmt.Sprintf("/v1/workspaces/%s/objects/%s:search", c.workspace, args[0])
			out, err := c.postJSON(context.Background(), path, spec)
			if err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), string(out))
			return nil
		},
	}
	cmd.Flags().StringVar(&where, "where", "", "filter shorthand: prop:eq:value,prop:gt:n")
	cmd.Flags().IntVar(&limit, "limit", 50, "page limit")
	return cmd
}

// parseShorthand turns "status:eq:open,amount:gt:100" into a Filter AST.
func parseShorthand(s string) (types.Filter, error) {
	parts := strings.Split(s, ",")
	children := make([]types.Filter, 0, len(parts))
	for _, p := range parts {
		bits := strings.SplitN(p, ":", 3)
		if len(bits) != 3 {
			return types.Filter{}, fmt.Errorf("invalid filter %q (want prop:op:value)", p)
		}
		children = append(children, types.Filter{
			Op:       types.FilterOp(bits[1]),
			Property: bits[0],
			Value:    bits[2],
		})
	}
	if len(children) == 1 {
		return children[0], nil
	}
	return types.Filter{Op: types.FilterOpAnd, Children: children}, nil
}
