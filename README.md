# jsonrpsee-server WebSocket extension hook

This is a server-only subtree of `jsonrpsee-server` 0.23.2 with one narrow
addition: a per-connection WebSocket extension factory exposed through the
server configuration and builder APIs.

The repository preserves the upstream history for the `server` package.
Extension negotiation and frame behavior remain owned by downstream
applications. See the [upstream jsonrpsee project](https://github.com/paritytech/jsonrpsee)
for the full workspace and documentation.
