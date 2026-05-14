// DataType is the canonical set of property types Lattice understands.
// Adapters translate these to their native storage types.

package types

import "fmt"

// DataType is the canonical type of a property value.
type DataType string

// The closed set of supported types. Adding a new type requires updates in
// every adapter implementation and in the codegen generators.
const (
	DataTypeString      DataType = "string"
	DataTypeInteger     DataType = "integer"
	DataTypeBigInt      DataType = "bigint"
	DataTypeFloat       DataType = "float"
	DataTypeDecimal     DataType = "decimal"
	DataTypeBoolean     DataType = "boolean"
	DataTypeDate        DataType = "date"
	DataTypeTimestamp   DataType = "timestamp"
	DataTypeTimestampTZ DataType = "timestamptz"
	DataTypeUUID        DataType = "uuid"
	DataTypeJSON        DataType = "json"
	DataTypeGeometry    DataType = "geometry"
)

// dataTypeAttrs is the single source of truth for per-type metadata. Any
// classification helper should add a column here rather than introduce a
// switch statement elsewhere in the codebase.
var dataTypeAttrs = map[DataType]struct {
	numeric  bool
	temporal bool
}{
	DataTypeString:      {},
	DataTypeInteger:     {numeric: true},
	DataTypeBigInt:      {numeric: true},
	DataTypeFloat:       {numeric: true},
	DataTypeDecimal:     {numeric: true},
	DataTypeBoolean:     {},
	DataTypeDate:        {temporal: true},
	DataTypeTimestamp:   {temporal: true},
	DataTypeTimestampTZ: {temporal: true},
	DataTypeUUID:        {},
	DataTypeJSON:        {},
	DataTypeGeometry:    {},
}

// Validate reports whether dt is one of the recognized data types.
func (dt DataType) Validate() error {
	if _, ok := dataTypeAttrs[dt]; !ok {
		return fmt.Errorf("unknown data type %q", dt)
	}
	return nil
}

// IsNumeric reports whether values of this type support arithmetic comparisons
// (gt, gte, lt, lte) without conversion.
func (dt DataType) IsNumeric() bool { return dataTypeAttrs[dt].numeric }

// IsTemporal reports whether values of this type are dates or timestamps.
func (dt DataType) IsTemporal() bool { return dataTypeAttrs[dt].temporal }

// String returns the wire form of dt.
func (dt DataType) String() string { return string(dt) }
