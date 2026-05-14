// Reflection-based ontology registration. Read once at startup; the runtime
// then operates on the validated, materialized ontology. Tag grammar:
//
//	lattice:"<api_name>[,token...]"
//
// Tokens are interpreted as:
//   - "primary_key"     → field is the primary key
//   - "indexed"         → property is queryable / has an index hint
//   - "nullable"        → property may be NULL
//   - "uuid", "bigint", "decimal", ... → override the inferred DataType
//   - "pii", "secret", ... → property tags (any unrecognized token is a tag)
//   - key=value forms: "tag=pii", "name=email_address" (for renaming)
//
// First positional token is the api_name; everything else is a flag or
// key=value. Tag tokens with `=` use the explicit form; bare tokens are
// matched against the keyword list above and otherwise treated as a tag.

package lattice

import (
	"fmt"
	"reflect"
	"strings"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// fieldSpec is the parsed result of one struct field's lattice tag.
type fieldSpec struct {
	APIName       string
	DataType      types.DataType
	PrimaryKey    bool
	Indexed       bool
	Nullable      bool
	Tags          []string
	AllowedValues []string
	Markings      []string
	Skip          bool // unexported or `lattice:"-"`
}

// parseFieldSpec inspects a reflect.StructField and returns its fieldSpec.
// The caller passes the raw `lattice` tag value if any; this function does
// not call Lookup itself so callers can override with custom tag names.
func parseFieldSpec(f reflect.StructField) fieldSpec {
	if !f.IsExported() {
		return fieldSpec{Skip: true}
	}
	raw, _ := f.Tag.Lookup("lattice")
	if raw == "-" {
		return fieldSpec{Skip: true}
	}

	spec := fieldSpec{
		APIName:  defaultAPIName(f.Name),
		DataType: inferDataType(f.Type),
	}
	if raw == "" {
		return spec
	}
	tokens := strings.Split(raw, ",")
	for i, tok := range tokens {
		tok = strings.TrimSpace(tok)
		if tok == "" {
			continue
		}
		if i == 0 && !strings.Contains(tok, "=") && !isKnownToken(tok) {
			spec.APIName = tok
			continue
		}
		applyToken(&spec, tok)
	}
	return spec
}

// applyToken interprets one token from the tag and mutates spec accordingly.
func applyToken(spec *fieldSpec, tok string) {
	if k, v, ok := splitKV(tok); ok {
		applyKV(spec, k, v)
		return
	}
	switch tok {
	case "primary_key", "pk":
		spec.PrimaryKey = true
	case "indexed":
		spec.Indexed = true
	case "nullable":
		spec.Nullable = true
	case "uuid":
		spec.DataType = types.DataTypeUUID
	case "bigint":
		spec.DataType = types.DataTypeBigInt
	case "decimal":
		spec.DataType = types.DataTypeDecimal
	case "json":
		spec.DataType = types.DataTypeJSON
	case "geometry":
		spec.DataType = types.DataTypeGeometry
	case "date":
		spec.DataType = types.DataTypeDate
	case "timestamp":
		spec.DataType = types.DataTypeTimestamp
	case "timestamptz":
		spec.DataType = types.DataTypeTimestampTZ
	default:
		// Anything we don't recognize is a property tag (pii, secret, etc.).
		spec.Tags = append(spec.Tags, tok)
	}
}

// applyKV handles key=value forms in the tag.
func applyKV(spec *fieldSpec, k, v string) {
	switch k {
	case "name":
		spec.APIName = v
	case "tag":
		spec.Tags = append(spec.Tags, v)
	case "allowed_values":
		spec.AllowedValues = strings.Split(v, "|")
	case "marking":
		spec.Markings = append(spec.Markings, v)
	}
}

// isKnownToken reports whether tok is a reserved flag/datatype keyword.
// Used to disambiguate the first positional token (api_name) from a flag.
func isKnownToken(tok string) bool {
	switch tok {
	case "primary_key", "pk", "indexed", "nullable",
		"uuid", "bigint", "decimal", "json", "geometry",
		"date", "timestamp", "timestamptz":
		return true
	}
	return false
}

// inferDataType maps a Go reflect.Type to a Lattice DataType.
// Override via tag tokens (e.g., explicit "uuid" on a string field).
func inferDataType(t reflect.Type) types.DataType {
	for t.Kind() == reflect.Ptr {
		t = t.Elem()
	}
	switch t.Kind() {
	case reflect.String:
		return types.DataTypeString
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32:
		return types.DataTypeInteger
	case reflect.Int64, reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return types.DataTypeBigInt
	case reflect.Float32, reflect.Float64:
		return types.DataTypeFloat
	case reflect.Bool:
		return types.DataTypeBoolean
	case reflect.Struct:
		if t == reflect.TypeOf(time.Time{}) {
			return types.DataTypeTimestampTZ
		}
		return types.DataTypeJSON
	case reflect.Slice, reflect.Array, reflect.Map:
		return types.DataTypeJSON
	}
	return types.DataTypeString
}

// defaultAPIName converts an exported Go field name (CamelCase) to the
// canonical snake_case api_name. ID stays "id"; HTTPCode → "http_code".
func defaultAPIName(name string) string {
	var b strings.Builder
	for i, r := range name {
		if i > 0 && r >= 'A' && r <= 'Z' {
			prev := name[i-1]
			if prev >= 'a' && prev <= 'z' {
				b.WriteByte('_')
			} else if i+1 < len(name) {
				next := name[i+1]
				if next >= 'a' && next <= 'z' {
					b.WriteByte('_')
				}
			}
		}
		if r >= 'A' && r <= 'Z' {
			b.WriteRune(r + 32)
		} else {
			b.WriteRune(r)
		}
	}
	return b.String()
}

// splitKV splits a "k=v" token, returning ok=false when '=' is absent.
func splitKV(s string) (k, v string, ok bool) {
	idx := strings.IndexByte(s, '=')
	if idx < 0 {
		return "", "", false
	}
	return s[:idx], s[idx+1:], true
}

// describeStruct walks a struct type and returns the parsed field specs.
// Returns an error if the type is not a struct.
func describeStruct(t reflect.Type) ([]fieldSpec, error) {
	for t.Kind() == reflect.Ptr {
		t = t.Elem()
	}
	if t.Kind() != reflect.Struct {
		return nil, fmt.Errorf("lattice: %s is not a struct", t.String())
	}
	specs := make([]fieldSpec, 0, t.NumField())
	for i := 0; i < t.NumField(); i++ {
		spec := parseFieldSpec(t.Field(i))
		if spec.Skip {
			continue
		}
		specs = append(specs, spec)
	}
	return specs, nil
}
