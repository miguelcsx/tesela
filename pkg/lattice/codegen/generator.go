// Generator is the lookup table from language → renderer. Each renderer
// emits a zip archive bytes.

package codegen

import (
	"archive/zip"
	"bytes"
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Renderer turns an ontology snapshot into a multi-file SDK package.
type Renderer interface {
	Language() string
	Render(snap *types.Ontology) (map[string]string, error)
}

// Generators is the declarative renderer registry.
var Generators = map[string]Renderer{
	"typescript": &typescriptRenderer{},
	"python":     &pythonRenderer{},
	"go":         &goRenderer{},
	"rust":       &rustRenderer{},
}

// Generate returns a zipped SDK for snap in the chosen language.
func Generate(language string, snap *types.Ontology) ([]byte, error) {
	r, ok := Generators[strings.ToLower(language)]
	if !ok {
		return nil, fmt.Errorf("codegen: unsupported language %q", language)
	}
	files, err := r.Render(snap)
	if err != nil {
		return nil, err
	}
	return zipFiles(files)
}

func zipFiles(files map[string]string) ([]byte, error) {
	var buf bytes.Buffer
	w := zip.NewWriter(&buf)
	for name, content := range files {
		f, err := w.Create(name)
		if err != nil {
			return nil, err
		}
		if _, err := f.Write([]byte(content)); err != nil {
			return nil, err
		}
	}
	if err := w.Close(); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
