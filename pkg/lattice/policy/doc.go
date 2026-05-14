// Package policy is the rule evaluator that decides whether an actor is
// allowed to perform an operation against a resource, with what additional
// row filter, and with which property redactions.
//
// Rules are loaded from the ontology snapshot. Role inheritance is resolved
// at load time so the runtime evaluator never walks the inheritance graph.
// CEL expressions are compiled once per rule and cached on the loaded
// representation.
//
// The evaluator returns a Decision struct: Allow (bool), Filter (extra row
// predicate to AND with the request filter), and Redactions (property names
// to drop from the response). Conflicts follow deny-overrides: a single
// matching deny rule overrides any number of allows.
package policy
