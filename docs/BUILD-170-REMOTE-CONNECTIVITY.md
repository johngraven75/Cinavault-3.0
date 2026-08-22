# CinaVault Premium Build 170 — Automatic Remote Connectivity

Build 170 adds automatic direct-connect NAT traversal and a secure outbound cloud relay fallback for the embedded CinaVault media server.

## Connectivity order

1. Start the embedded server on port `32400`.
2. Attempt automatic UPnP TCP port mapping.
3. Attempt NAT-PMP TCP port mapping when UPnP is unavailable.
4. Verify the public address.
5. Start an outbound Cloudflare Tunnel when direct mapping fails or relay preference is enabled.
6. Continue to enforce CinaVault account-password, access-key, session-token, and permission checks at the embedded server.

## Relay modes

- **Named production tunnel:** set `CINAVAULT_CLOUDFLARE_TUNNEL_TOKEN` and `CINAVAULT_CLOUDFLARE_PUBLIC_URL` before launching CinaVault.
- **Automatic zero-configuration fallback:** when no named-tunnel token is configured, CinaVault can start a Cloudflare Quick Tunnel and discover the generated HTTPS URL automatically.

Cloudflare Quick Tunnels have no uptime SLA and are intended as a zero-configuration fallback. A named tunnel and stable hostname are required for a production-operated relay endpoint.

## Security

The relay transports requests to the local embedded server; it does not bypass authorization. Remote clients must authenticate with a valid CinaVault account or access key and receive the required permissions before library or streaming endpoints are served.
