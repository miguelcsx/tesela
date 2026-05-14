// Sealer is the AEAD used to encrypt secrets at rest. The default and only
// implementation is AESGCMSealer.

package crypto

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
)

// NonceSize is the byte length of the random nonce prepended to every
// ciphertext produced by AESGCMSealer.
const NonceSize = 12

// Sealer encrypts and decrypts opaque byte payloads using authenticated
// encryption. Implementations are safe for concurrent use.
type Sealer interface {
	Seal(plaintext []byte) ([]byte, error)
	Open(ciphertext []byte) ([]byte, error)
}

// AESGCMSealer is the default Sealer implementation.
type AESGCMSealer struct {
	aead cipher.AEAD
}

// NewAESGCMSealer constructs a sealer using the supplied raw key. The key
// length selects the cipher (16=AES-128, 24=AES-192, 32=AES-256). Other
// lengths are rejected.
func NewAESGCMSealer(key []byte) (*AESGCMSealer, error) {
	if err := validateKeyLength(len(key)); err != nil {
		return nil, err
	}
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, fmt.Errorf("aes cipher: %w", err)
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, fmt.Errorf("gcm: %w", err)
	}
	return &AESGCMSealer{aead: aead}, nil
}

// Seal returns nonce || ciphertext for plaintext.
func (s *AESGCMSealer) Seal(plaintext []byte) ([]byte, error) {
	nonce := make([]byte, NonceSize)
	if _, err := rand.Read(nonce); err != nil {
		return nil, fmt.Errorf("nonce: %w", err)
	}
	// AEAD.Seal appends ciphertext+tag to the destination slice; we use the
	// nonce slice itself as the prefix.
	return s.aead.Seal(nonce, nonce, plaintext, nil), nil
}

// Open recovers plaintext from a ciphertext previously produced by Seal.
func (s *AESGCMSealer) Open(blob []byte) ([]byte, error) {
	if len(blob) < NonceSize {
		return nil, errors.New("ciphertext shorter than nonce")
	}
	nonce, ct := blob[:NonceSize], blob[NonceSize:]
	pt, err := s.aead.Open(nil, nonce, ct, nil)
	if err != nil {
		return nil, fmt.Errorf("aead open: %w", err)
	}
	return pt, nil
}

// KeyFromHex decodes a hex-encoded KEK into raw bytes. Returns an error if
// the result does not have a valid AES key length (16/24/32 bytes).
func KeyFromHex(s string) ([]byte, error) {
	raw, err := hex.DecodeString(s)
	if err != nil {
		return nil, fmt.Errorf("hex decode: %w", err)
	}
	if err := validateKeyLength(len(raw)); err != nil {
		return nil, err
	}
	return raw, nil
}

// validateKeyLength returns nil for 16/24/32-byte keys.
func validateKeyLength(n int) error {
	switch n {
	case 16, 24, 32:
		return nil
	default:
		return fmt.Errorf("aes key must be 16/24/32 bytes, got %d", n)
	}
}
