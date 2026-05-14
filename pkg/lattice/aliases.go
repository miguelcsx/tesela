// Package lattice is the high-level facade. Users import this package and
// use the fluent App builder to define their ontology, register backends,
// and produce an http.Handler. All other sub-packages are internal building
// blocks; advanced users can still import them directly.
//
// Hello world (≤25 líneas):
//
//	package main
//
//	import (
//	    "context"
//	    "log"
//	    "net/http"
//	    "github.com/miguelcsx/lattice/pkg/lattice"
//	)
//
//	func main() {
//	    app := lattice.New()
//	    app.ObjectType("Customer").
//	        Property("id", lattice.UUID).PrimaryKey().
//	        Property("email", lattice.String).Tag("pii").
//	        Search(searchCustomers)
//	    app.Allow("admin").Operations(lattice.AllOps...)
//	    log.Fatal(http.ListenAndServe(":8080", app.Handler()))
//	}
//
//	func searchCustomers(ctx context.Context, q lattice.Query) (lattice.Page, error) {
//	    return lattice.Page{Records: []lattice.Record{...}}, nil
//	}

package lattice

import "github.com/miguelcsx/lattice/pkg/lattice/types"

// Re-exports of the most commonly used domain types. Users rarely need to
// reach into pkg/lattice/types directly.
type (
	// Record is one row of values keyed by property api_name.
	Record = types.Record
	// Page is a paginated result of records.
	Page = types.Page
	// Query is the resolved query handed to a Search backend (filter + sort + page).
	Query = types.QuerySpec
	// Filter is the predicate AST.
	Filter = types.Filter
	// Mutation is the resolved write operation.
	Mutation = types.Mutation
	// MutationResult is what a Mutator returns.
	MutationResult = types.MutationResult
	// Actor is the principal performing an operation.
	Actor = types.Actor
	// SourceConfig points at a table/view in the user's backend.
	SourceConfig = types.SourceConfig
	// ObjectType describes one kind of operational entity.
	ObjectType = types.ObjectType
	// Operation is the kind of access being requested.
	Operation = types.Operation
	// APIName is the user-facing identifier used as the key for property lookups.
	APIName = types.APIName
)

// DataType constants — used as Property("id", lattice.UUID).
const (
	String      = types.DataTypeString
	Integer     = types.DataTypeInteger
	BigInt      = types.DataTypeBigInt
	Float       = types.DataTypeFloat
	Decimal     = types.DataTypeDecimal
	Boolean     = types.DataTypeBoolean
	Date        = types.DataTypeDate
	Timestamp   = types.DataTypeTimestamp
	TimestampTZ = types.DataTypeTimestampTZ
	UUID        = types.DataTypeUUID
	JSON        = types.DataTypeJSON
	Geometry    = types.DataTypeGeometry
)

// Operation constants — used as app.Allow("admin").Operations(lattice.Read, lattice.Search).
const (
	Read      = types.OperationRead
	Search    = types.OperationSearch
	Aggregate = types.OperationAggregate
	Traverse  = types.OperationTraverse
	Create    = types.OperationCreate
	Update    = types.OperationUpdate
	Delete    = types.OperationDelete
	Execute   = types.OperationExecute
)

// AllOps is the set of every operation.
var AllOps = []types.Operation{Read, Search, Aggregate, Traverse, Create, Update, Delete, Execute}
