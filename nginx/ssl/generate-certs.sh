#!/bin/bash

# Generate self-signed SSL certificates for development
# DO NOT use these in production!

echo "Generating self-signed SSL certificates for development..."

# Create private key
openssl genrsa -out key.pem 2048

# Create certificate signing request
openssl req -new -key key.pem -out cert.csr -subj "/C=US/ST=CA/L=San Francisco/O=LinkWithMentor/OU=Development/CN=localhost"

# Create certificate
openssl x509 -req -in cert.csr -signkey key.pem -out cert.pem -days 365 -extensions v3_req -extfile <(
cat <<EOF
[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = linkwithmentor.local
DNS.3 = *.linkwithmentor.local
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
)

# Clean up
rm cert.csr

echo "SSL certificates generated:"
echo "  - cert.pem (certificate)"
echo "  - key.pem (private key)"
echo ""
echo "⚠️  These are self-signed certificates for development only!"
echo "   Browsers will show security warnings."
echo "   For production, use certificates from a trusted CA."