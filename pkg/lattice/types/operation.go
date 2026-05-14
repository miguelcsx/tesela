// Operation is the closed set of operations the policy engine evaluates.

package types

import "fmt"

// Operation names a category of action over a resource for the policy engine.
type Operation string

const (
	OperationRead          Operation = "read"           // single object by primary key
	OperationSearch        Operation = "search"         // multi-object query with filters
	OperationAggregate     Operation = "aggregate"      // group-by / metric query
	OperationTraverse      Operation = "traverse"       // follow a link from a source object
	OperationCreate        Operation = "create"         // create a new object
	OperationUpdate        Operation = "update"         // update an existing object
	OperationDelete        Operation = "delete"         // delete an existing object
	OperationExecute       Operation = "execute"        // execute an action type
	OperationUpload        Operation = "upload"         // initiate an upload
	OperationApproveUpload Operation = "approve_upload" // approve mapping / trigger load
	OperationReadUpload    Operation = "read_upload"    // get upload status / list
	OperationDeleteUpload  Operation = "delete_upload"  // cancel / rollback upload
)

// operationAttrs centralizes per-operation classification. Add a column here
// rather than writing switch statements at the call site.
var operationAttrs = map[Operation]struct {
	read bool
}{
	OperationRead:      {read: true},
	OperationSearch:    {read: true},
	OperationAggregate: {read: true},
	OperationTraverse:  {read: true},
	OperationCreate:    {},
	OperationUpdate:    {},
	OperationDelete:    {},
	OperationExecute:   {},
}

// Validate reports whether op is one of the recognized operations.
func (op Operation) Validate() error {
	if _, ok := operationAttrs[op]; !ok {
		return fmt.Errorf("unknown operation %q", op)
	}
	return nil
}

// IsRead reports whether the operation is a non-mutating read.
func (op Operation) IsRead() bool { return operationAttrs[op].read }

// String implements fmt.Stringer.
func (op Operation) String() string { return string(op) }
