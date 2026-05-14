package types_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestDataType_Validate_KnownTypes(t *testing.T) {
	t.Parallel()

	known := []types.DataType{
		types.DataTypeString, types.DataTypeInteger, types.DataTypeBigInt,
		types.DataTypeFloat, types.DataTypeDecimal, types.DataTypeBoolean,
		types.DataTypeDate, types.DataTypeTimestamp, types.DataTypeTimestampTZ,
		types.DataTypeUUID, types.DataTypeJSON, types.DataTypeGeometry,
	}
	for _, dt := range known {
		if err := dt.Validate(); err != nil {
			t.Fatalf("Validate(%q) returned %v, want nil", dt, err)
		}
	}
}

func TestDataType_Validate_RejectsUnknown(t *testing.T) {
	t.Parallel()

	for _, dt := range []types.DataType{"", "blob", "money"} {
		if err := dt.Validate(); err == nil {
			t.Fatalf("Validate(%q) returned nil, want error", dt)
		}
	}
}

func TestDataType_IsNumeric(t *testing.T) {
	t.Parallel()

	cases := map[types.DataType]bool{
		types.DataTypeInteger:  true,
		types.DataTypeBigInt:   true,
		types.DataTypeFloat:    true,
		types.DataTypeDecimal:  true,
		types.DataTypeString:   false,
		types.DataTypeBoolean:  false,
		types.DataTypeDate:     false,
		types.DataTypeJSON:     false,
		types.DataTypeGeometry: false,
	}
	for dt, want := range cases {
		if got := dt.IsNumeric(); got != want {
			t.Fatalf("IsNumeric(%q) = %v, want %v", dt, got, want)
		}
	}
}

func TestDataType_IsTemporal(t *testing.T) {
	t.Parallel()

	cases := map[types.DataType]bool{
		types.DataTypeDate:        true,
		types.DataTypeTimestamp:   true,
		types.DataTypeTimestampTZ: true,
		types.DataTypeString:      false,
		types.DataTypeInteger:     false,
	}
	for dt, want := range cases {
		if got := dt.IsTemporal(); got != want {
			t.Fatalf("IsTemporal(%q) = %v, want %v", dt, got, want)
		}
	}
}
