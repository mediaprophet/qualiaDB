# QDNF Legacy Internet Gateway

**Short name:** LIG
**Status:** Normative design 0.1

## 1. Purpose

The Legacy Internet Gateway gives a QDNF-native node access to DNS/IP/TLS/HTTP services without
making those systems dependencies of QDNF naming, routing, identity, or authorization.

The gateway is an application proxy and policy boundary, not a transparent default router.

```text
QDNF native side                    Gateway                     Legacy side

QResolve/QPolicy/QSession  <->  explicit translation  <->  DNS/IP/TCP/UDP/TLS/HTTP
native identifiers                audit + consent             domains and addresses
```

## 2. Separation requirements

- Native QDNF traffic MUST NOT be forwarded to the LIG merely because QResolve failed.
- Legacy access requires an explicit `http:`, `https:`, `dns:`, `ip:`, or application-approved
  legacy target.
- The LIG has separate route, cache, key, log, and policy namespaces.
- DNS answers never populate the native RAR cache.
- Native Alias Assertions never become DNS answers unless an administrator explicitly publishes a
  bridge record.
- A legacy Web PKI certificate does not authorize a DID route update.
- A DID/RAR proof does not assert control of a DNS name unless separately proven.
- The UI visibly marks the boundary and the legacy authority used.
- A gateway may be self-hosted, community-operated, or remote, but no particular gateway is mandatory.

## 3. Gateway services

| Service | Native request | Legacy action | Returned evidence |
|---|---|---|---|
| Web fetch | explicit URL and method | DNS, IP routing, TLS, HTTP | final URL, DNS/DNSSEC state if available, TLS chain state, response digest |
| Legacy socket | host/IP, port, protocol, purpose | controlled TCP/UDP proxy | resolved addresses, policy, byte/accounting receipt |
| DNS inspect | exact name/type | DNS query only | full bounded answer and validation metadata |
| DID-over-web | DID method/service request | HTTPS according to method rules | method result, Web PKI evidence, fetched digest |
| QDNF bootstrap import | explicit URL/QR/file | fetch signed RAR/invitation | QDNF proof result independent of HTTPS |
| Public mirror publish | authorized resource | upload through configured service | remote receipt; no claim of global persistence |

The base LIG does not proxy arbitrary inbound legacy connections. Inbound publication is a separate,
explicit service with its own capability and exposure warning.

## 4. Request protocol

The native service reference is `qdnf:service:legacy-gateway`. A request contains:

- request ID and expiry;
- exact legacy scheme and target;
- method/protocol and bounded headers;
- purpose and requesting qapp/agent;
- maximum response bytes, redirects, duration, and cost;
- cookie/auth partition reference, never raw secrets when a vault handle suffices;
- expected content digest or controller proof when known; and
- capability authorizing gateway use.

The LIG returns:

- outcome and stable reason;
- normalized final target and redirect chain;
- DNS resolution and validation metadata;
- TLS peer/chain evidence;
- byte, time, and optional cost accounting;
- response content or an encrypted content-store reference;
- response strong digest; and
- gateway signature over the receipt.

The gateway signature proves what the gateway observed, not the truth or safety of remote content.

## 5. Trust model

There are three independent statements:

1. **Legacy transport statement:** DNS/Web PKI/IP connected this gateway to a legacy endpoint.
2. **Gateway observation statement:** this gateway claims it received these bytes/evidence.
3. **QDNF resource statement:** a DID/content controller authorizes or hashes the requested resource.

Clients display and evaluate them separately. A successful HTTPS request can remain `legacy-verified`
without becoming `qdnf-controller-verified`. A QDNF content digest can verify bytes received through
an otherwise untrusted gateway.

For sensitive retrieval, clients may compare multiple independent gateways or require a known
content digest. Multiple matching gateways improve evidence but are not mathematical proof of truth.

## 6. DNS behavior

DNS exists only on the LIG's legacy side.

- The gateway uses configured recursive resolvers, local validation, or both.
- DNSSEC validation state is reported exactly; insecure and bogus are distinct.
- Search suffixes and implicit name completion are disabled for protocol requests.
- Responses are bounded, cached only in the legacy namespace, and obey TTL caps.
- Private QDNF identifiers and aliases are never leaked as DNS query names.
- A DNS TXT record may carry a QDNF bootstrap pointer, but the enclosed RAR is independently verified.
- Failure to resolve a domain never triggers semantic guessing or a native-alias match without a new
  user action.

## 7. HTTP/TLS behavior

- Redirect count defaults to 5 and may not cross policy boundaries silently.
- TLS validation follows the selected legacy trust policy and reports custom-root use.
- Cookies, storage, credentials, and client certificates are partitioned by requesting habitat/qapp
  and legacy origin.
- Content is untrusted input even after TLS succeeds.
- Active content runs only in the sandboxed browser profile, not in the resolver or QDNF daemon.
- Semantic ingestion records source URL, fetch time, gateway, digest, transformation, and epistemic
  status.
- A downloaded Q42 artifact is verified by its Q42 manifest/digest before native use.

## 8. Inbound bridge

Publishing a QDNF service to the legacy Internet requires an explicit Inbound Bridge Grant defining:

- native target and service;
- public hostname/address or relay account;
- allowed legacy methods and maximum body/rate;
- authentication mapping;
- data sensitivity ceiling;
- start, expiry, revocation, and accountable controller;
- whether aliases/search indexing are allowed; and
- required audit/abuse controls.

The gateway terminates the legacy connection, constructs a QDNF operation, and presents it to
QPolicy. Source IP, domain, or TLS client identity is evidence only; it is never automatically a
native DID or capability.

## 9. Failure and downgrade

If a native target has no verified route, the client reports native failure. It may offer a clearly
labelled legacy search as a separate action. It MUST NOT:

- append a TLD;
- send the target string to a search engine or DNS resolver;
- try HTTP after QDNF proof failure;
- accept an old RAR embedded in a webpage over a newer withdrawal; or
- treat gateway reachability as native-network availability.

Legacy-to-native upgrade is also explicit: the client fetches a signed RAR/identifier proof, verifies
it through QResolve, shows the new controller and permissions, and asks before creating a relationship
or persistent route.

## 10. Privacy

- Prefer a local gateway for sensitive browsing.
- Remote gateways see requested legacy destinations unless an additional privacy transport is used.
- QDNF relationship graphs, private aliases, and exact native target identifiers are not included in
  legacy requests unless required and authorized.
- Gateway logs default to digests and accounting, not response bodies, cookies, credentials, or
  private identifier strings.
- Users can select no-log, local-only, or accountable-retention policies where implementations offer
  them; labels must describe actual behavior rather than make unverifiable promises.

## 11. Conformance profiles

- `LIG-Web`: explicit DNS/IP/TLS/HTTP fetch with bounded evidence.
- `LIG-Socket`: controlled TCP/UDP egress proxy.
- `LIG-Bootstrap`: imports independently verifiable QDNF invitations/RARs.
- `LIG-Publish`: capability-gated inbound exposure.

A `QDNF-Native-Independent` node need not implement any LIG profile. Removing or disconnecting every
LIG must leave native QLink, QRoute, QResolve, QPolicy, QSession, and QSync operational.
