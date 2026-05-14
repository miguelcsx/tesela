// Package crypto provides authenticated symmetric encryption used to seal
// credentials at rest in the metadata database.
//
// The sole supported AEAD is AES-GCM (128/192/256-bit keys, depending on
// the supplied KEK length). Keys are normally loaded by the secret provider
// at process startup; rotation is performed by re-sealing every datasource
// with the new key, which is a one-pass migration not yet implemented.
//
// The wire format produced by Seal is `nonce || ciphertext` where nonce is
// NonceSize bytes (12). This format is intentionally minimal — there is no
// version byte, because rotation will encrypt with a new key and the keying
// material is identified out-of-band by the SecretProvider reference, not
// embedded in the ciphertext.
package crypto
