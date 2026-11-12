package main

import (
	"context"
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/hex"
	"fmt"
	"math/big"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/nbd-wtf/go-nostr"
	"github.com/nbd-wtf/go-nostr/nip19"
	"github.com/zapstore/zsp/v2"
)

func TestSignProofMatchesZspMessage(t *testing.T) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	cert := &x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject:      pkix.Name{CommonName: "test"},
		NotBefore:    time.Unix(0, 0),
		NotAfter:     time.Unix(1<<32, 0),
	}
	cert.Raw = []byte("test certificate")

	const pubkey = "0000000000000000000000000000000000000000000000000000000000000001"
	signer := keystoreProofSigner{key: key, cert: cert, pubkey: pubkey}
	signed, err := signer.SignProof(context.Background(), 1000, 2000)
	if err != nil {
		t.Fatal(err)
	}
	if signed.PublicKey != pubkey {
		t.Fatalf("pubkey = %q", signed.PublicKey)
	}
	hash := sha256.Sum256(cert.Raw)
	if signed.CertificateHash != hex.EncodeToString(hash[:]) {
		t.Fatalf("certificate hash = %q", signed.CertificateHash)
	}

	message := fmt.Sprintf(
		"Verifying at %d until %d that I control the following Nostr public key: %s",
		1000, 2000, pubkey,
	)
	digest := sha256.Sum256([]byte(message))
	if err := rsa.VerifyPKCS1v15(&key.PublicKey, crypto.SHA256, digest[:], signed.Signature); err != nil {
		t.Fatalf("signature does not verify: %v", err)
	}
}

func TestNostrPubkey(t *testing.T) {
	secret := nostr.GeneratePrivateKey()
	nsec, err := nip19.EncodePrivateKey(secret)
	if err != nil {
		t.Fatal(err)
	}
	want, err := nostr.GetPublicKey(secret)
	if err != nil {
		t.Fatal(err)
	}

	for _, input := range []string{nsec, secret, strings.ToUpper(secret)} {
		got, err := nostrPubkey(input)
		if err != nil {
			t.Fatalf("nostrPubkey(%q): %v", input, err)
		}
		if got != want {
			t.Fatalf("nostrPubkey(%q) = %q, want %q", input, got, want)
		}
	}

	bunker := "bunker://" + strings.ToUpper(want) + "?relay=wss%3A%2F%2Frelay.example"
	got, err := nostrPubkey(bunker)
	if err != nil {
		t.Fatalf("nostrPubkey(bunker): %v", err)
	}
	if got != want {
		t.Fatalf("nostrPubkey(bunker) = %q, want %q", got, want)
	}
}

func TestRepoConfigs(t *testing.T) {
	cases := []struct {
		path     string
		asset    string
		notAsset string
	}{
		{"../../zapstore.yaml", "minifrost_coordinator-1.0.0-android.apk", "minifrost_signer-1.0.0-android.apk"},
		{"../../zapstore-signer.yaml", "minifrost_signer-1.0.0-android.apk", "minifrost_coordinator-1.0.0-android.apk"},
	}
	for _, tc := range cases {
		config, err := zsp.LoadConfig(tc.path)
		if err != nil {
			t.Fatalf("LoadConfig(%q): %v", tc.path, err)
		}
		if config.Fetch.Repository == "" {
			t.Fatalf("%s has no repository", tc.path)
		}
		match, err := regexp.Compile(config.Fetch.Match)
		if err != nil {
			t.Fatalf("%s match: %v", tc.path, err)
		}
		if !match.MatchString(tc.asset) {
			t.Errorf("%s match %q does not select %s", tc.path, config.Fetch.Match, tc.asset)
		}
		if match.MatchString(tc.notAsset) {
			t.Errorf("%s match %q also selects %s", tc.path, config.Fetch.Match, tc.notAsset)
		}
		filter, err := regexp.Compile(config.Fetch.ReleaseFilter)
		if err != nil {
			t.Fatalf("%s release_filter: %v", tc.path, err)
		}
		if !filter.MatchString("v1.0.0") {
			t.Errorf("%s release_filter %q does not match v1.0.0", tc.path, config.Fetch.ReleaseFilter)
		}
	}
}
