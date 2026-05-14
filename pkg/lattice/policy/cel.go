// CEL environment + program cache used by the evaluator. Compilation is
// expensive enough that we cache by expression string; evaluation reuses the
// pre-compiled program.

package policy

import (
	"fmt"
	"sync"

	"github.com/google/cel-go/cel"
)

// celEnv is the singleton CEL environment used by the policy evaluator.
// Variables: actor (object with user_id, workspace_id, roles, claims),
// resource (object with kind, api_name, subject_key), input (mutation input
// for action evaluations), now (timestamp).
var celEnv = mustCELEnv()

func mustCELEnv() *cel.Env {
	env, err := cel.NewEnv(
		cel.Variable("actor", cel.DynType),
		cel.Variable("resource", cel.DynType),
		cel.Variable("input", cel.DynType),
		cel.Variable("subject", cel.DynType),
		cel.Variable("now", cel.DynType),
	)
	if err != nil {
		panic(fmt.Sprintf("policy: cel env: %v", err))
	}
	return env
}

// programCache memoizes compiled CEL programs by expression text.
type programCache struct {
	mu   sync.RWMutex
	data map[string]cel.Program
}

func newProgramCache() *programCache { return &programCache{data: make(map[string]cel.Program)} }

func (c *programCache) get(expr string) (cel.Program, error) {
	c.mu.RLock()
	if prg, ok := c.data[expr]; ok {
		c.mu.RUnlock()
		return prg, nil
	}
	c.mu.RUnlock()

	c.mu.Lock()
	defer c.mu.Unlock()
	if prg, ok := c.data[expr]; ok {
		return prg, nil
	}
	ast, iss := celEnv.Compile(expr)
	if iss != nil && iss.Err() != nil {
		return nil, fmt.Errorf("compile %q: %w", expr, iss.Err())
	}
	prg, err := celEnv.Program(ast)
	if err != nil {
		return nil, fmt.Errorf("program: %w", err)
	}
	c.data[expr] = prg
	return prg, nil
}
