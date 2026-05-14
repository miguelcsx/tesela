package upload

import (
	"context"
	"strings"
	"testing"
)

func TestDetectCSVProducesColumnStats(t *testing.T) {
	body := strings.NewReader("id,active,score\n1,true,10.5\n2,false,11.0\n3,true,\n")
	schema, err := Detect(context.Background(), "csv", body)
	if err != nil {
		t.Fatalf("detect csv: %v", err)
	}
	if got, want := len(schema.Columns), 3; got != want {
		t.Fatalf("expected %d columns, got %d", want, got)
	}
	if schema.Columns[0].DistinctCount == 0 {
		t.Fatal("expected distinct count to be populated")
	}
	if schema.Columns[0].TypeConfidence == 0 {
		t.Fatal("expected type confidence to be populated")
	}
}
