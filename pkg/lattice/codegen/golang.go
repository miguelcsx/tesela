// Go renderer — emits a tiny typed client.

package codegen

import (
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type goRenderer struct{}

func (*goRenderer) Language() string { return "go" }

func (r *goRenderer) Render(snap *types.Ontology) (map[string]string, error) {
	files := make(map[string]string, 3)
	files["go.mod"] = "module example.com/lattice-sdk\n\ngo 1.23\n"
	files["client.go"] = renderGoModule(snap)
	files["README.md"] = "# Lattice Go SDK\n"
	return files, nil
}

func renderGoModule(snap *types.Ontology) string {
	var b strings.Builder
	b.WriteString(`// Auto-generated Go SDK for Lattice.
package latticesdk

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
)

`)
	for _, ot := range snap.ObjectTypes {
		b.WriteString(fmt.Sprintf("type %s struct {\n", tsTypeName(string(ot.APIName))))
		for _, p := range ot.Properties {
			b.WriteString(fmt.Sprintf("\t%s %s `json:\"%s\"`\n", tsTypeName(string(p.APIName)), goType(p.DataType), p.APIName))
		}
		b.WriteString("}\n\n")
	}
	b.WriteString(`type Client struct {
	BaseURL, Token, Workspace string
	HTTP *http.Client
}

func NewClient(baseURL, token, workspace string) *Client {
	return &Client{BaseURL: baseURL, Token: token, Workspace: workspace, HTTP: http.DefaultClient}
}

func (c *Client) post(path string, body any, out any) error {
	raw, _ := json.Marshal(body)
	req, _ := http.NewRequest("POST", c.BaseURL+path, bytes.NewReader(raw))
	req.Header.Set("Authorization", "Bearer "+c.Token)
	req.Header.Set("Content-Type", "application/json")
	resp, err := c.HTTP.Do(req)
	if err != nil { return err }
	defer resp.Body.Close()
	if resp.StatusCode >= 400 { return fmt.Errorf("http %d", resp.StatusCode) }
	if out != nil { return json.NewDecoder(resp.Body).Decode(out) }
	return nil
}
`)
	return b.String()
}

func goType(dt types.DataType) string {
	switch dt {
	case types.DataTypeInteger:
		return "int"
	case types.DataTypeBigInt:
		return "int64"
	case types.DataTypeFloat, types.DataTypeDecimal:
		return "float64"
	case types.DataTypeBoolean:
		return "bool"
	default:
		return "string"
	}
}
