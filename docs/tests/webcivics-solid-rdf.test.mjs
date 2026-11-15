#!/usr/bin/env node
/**
 * VW-05 / QW-09 — Node parity fixtures against the pinned wasm-webcivics artifact.
 *
 * Loads docs/pkg/webcivics (not playground). Asserts Solid MIME serialize/parse
 * for Turtle, JSON-LD (compact), and N3, plus Accept negotiation and capability
 * advertisement for solid-rdf-media-types.
 *
 * Usage: node docs/tests/webcivics-solid-rdf.test.mjs
 */
import { createHash } from 'node:crypto';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { gunzipSync, gzipSync } from 'node:zlib';

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(here, '..', 'pkg', 'webcivics');
const wasmPath = join(pkgDir, 'qualia_webcivics_bg.wasm');
const jsPath = join(pkgDir, 'qualia.js');

function fail(msg) {
  console.error(`FAIL: ${msg}`);
  process.exitCode = 1;
}

function ok(msg) {
  console.log(`ok  ${msg}`);
}

function fnv1a64(str) {
  let h = 0xcbf29ce484222325n;
  const FNV = 0x100000001b3n;
  for (let i = 0; i < str.length; i++) {
    h ^= BigInt(str.charCodeAt(i));
    h = (h * FNV) & 0xffffffffffffffffn;
  }
  return h;
}

function packQuin(s, p, o) {
  return [s, p, o, 0n, 0n, 0n];
}

async function main() {
  if (!existsSync(wasmPath) || !existsSync(jsPath)) {
    fail(`missing webcivics package under ${pkgDir}`);
    return;
  }

  const bytes = readFileSync(wasmPath);
  const sha = createHash('sha256').update(bytes).digest('hex');
  const gz = gzipSync(bytes).length;
  ok(`artifact ${bytes.length} raw / ${gz} gzip sha256=${sha}`);

  const mod = await import(pathToFileURL(jsPath).href);
  if (typeof mod.initSync === 'function') {
    mod.initSync({ module: bytes });
  } else if (typeof mod.default === 'function') {
    await mod.default(bytes);
  } else {
    fail('no initSync/default on webcivics package');
    return;
  }

  const profile =
    typeof mod.compiled_profile === 'function'
      ? mod.compiled_profile()
      : typeof mod.get_engine_info === 'function'
        ? (mod.get_engine_info()?.profile ?? mod.get_engine_info()?.compiled_profile)
        : null;
  if (profile && profile !== 'webcivics') {
    fail(`compiled profile expected webcivics, got ${profile}`);
  } else if (profile) {
    ok(`compiled_profile=${profile}`);
  }

  const caps =
    typeof mod.list_capabilities_wasm === 'function'
      ? mod.list_capabilities_wasm()
      : [];
  const capList = Array.isArray(caps) ? caps : caps?.capabilities ?? [];
  for (const need of [
    'solid-rdf-media-types',
    'json-ld-compact-serialize',
    'rdf-serialization',
    'n3-parser',
    'turtle-parser',
    'json-ld-ingest',
    'json-ld-serialize',
  ]) {
    if (!capList.includes(need)) {
      fail(`capability missing: ${need} (got ${capList.length} caps)`);
    } else {
      ok(`capability ${need}`);
    }
  }

  if (typeof mod.serialize_rdf_wasm !== 'function') {
    fail('serialize_rdf_wasm missing');
    return;
  }
  if (typeof mod.parse_rdf_document_wasm !== 'function') {
    fail('parse_rdf_document_wasm missing — rebuild wasm-webcivics');
    return;
  }
  if (typeof mod.solid_negotiate_accept_wasm !== 'function') {
    fail('solid_negotiate_accept_wasm missing — rebuild wasm-webcivics');
    return;
  }
  if (typeof mod.export_solid_migration_wasm !== 'function') {
    fail('export_solid_migration_wasm missing — rebuild wasm-webcivics (solid-leave-migrate)');
    return;
  }

  const alice = fnv1a64('http://example.org/Alice');
  const knows = fnv1a64('http://example.org/knows');
  const bob = fnv1a64('http://example.org/Bob');
  const quins = [packQuin(alice, knows, bob)];

  const cases = [
    { format: 'text/turtle', expectCt: 'text/turtle', compact: false, needle: '<' },
    {
      format: 'application/ld+json',
      expectCt: 'application/ld+json',
      compact: true,
      needle: '@context',
    },
    { format: 'text/n3', expectCt: 'text/n3', compact: false, needle: '@prefix' },
  ];

  for (const c of cases) {
    const ser = mod.serialize_rdf_wasm({
      quins,
      format: c.format,
      compact: c.compact,
    });
    if (ser.content_type !== c.expectCt) {
      fail(`${c.format}: content_type=${ser.content_type}`);
      continue;
    }
    if (!String(ser.rdf_data).includes(c.needle)) {
      fail(`${c.format}: missing ${c.needle} in ${ser.rdf_data.slice(0, 120)}`);
      continue;
    }
    const parsed = mod.parse_rdf_document_wasm(ser.content_type, ser.rdf_data);
    const n = Number(parsed.quin_count);
    if (!(n >= 1)) {
      fail(`${c.format}: parse quin_count=${parsed.quin_count}`);
      continue;
    }
    ok(`${c.format} serialize→parse quin_count=${n}`);
  }

  const neg = mod.solid_negotiate_accept_wasm(
    'text/n3;q=0.5, application/ld+json, text/turtle;q=0.9',
  );
  if (neg.content_type !== 'application/ld+json') {
    fail(`Accept negotiation got ${neg.content_type}`);
  } else {
    ok(`Accept → ${neg.content_type}`);
  }

  const leave = mod.export_solid_migration_wasm({
    quins,
    sanctuary_choice: 'omit-sanctuary',
    grant_public_read: false,
    owner_webid: 'https://example.org/me#i',
  });
  if (!leave.turtle || !String(leave.turtle).includes('<')) {
    fail(`leave bundle turtle missing/invalid: ${String(leave.turtle).slice(0, 80)}`);
  } else if (!leave.manifest_jsonld || !String(leave.manifest_jsonld).includes('SolidMigrationBundle')) {
    fail('leave bundle manifest missing SolidMigrationBundle');
  } else if (!leave.acl || String(leave.acl).includes('foaf:Agent')) {
    fail('leave ACL must not grant public Read by default');
  } else if (Number(leave.stats?.exported) < 1) {
    fail(`leave stats.exported=${leave.stats?.exported}`);
  } else {
    ok(`solid-leave-migrate exported=${leave.stats.exported} turtle=${leave.turtle.length}B`);
  }

  if (typeof mod.plan_sanctuary_migration_wasm === 'function') {
    const notice = mod.plan_sanctuary_migration_wasm({ quins });
    if (notice.requires_choice !== false && notice.requires_choice !== true) {
      fail('plan_sanctuary_migration_wasm missing requires_choice');
    } else {
      ok(`sanctuary notice requires_choice=${notice.requires_choice}`);
    }
  } else {
    fail('plan_sanctuary_migration_wasm missing');
  }

  if (typeof mod.import_solid_to_qualia_backup_wasm === 'function') {
    const back = mod.import_solid_to_qualia_backup_wasm({
      resources: [
        { name: 'data.ttl', content_type: 'text/turtle', body: leave.turtle },
      ],
    });
    if (!back.backup_json || !String(back.backup_json).includes('webcivics.vault-backup.v1')) {
      fail('import_solid_to_qualia_backup_wasm missing vault-backup schema');
    } else {
      ok('solid-qualia-return backup package');
    }
  } else {
    fail('import_solid_to_qualia_backup_wasm missing');
  }

  const leaveCaps = typeof mod.list_capabilities_wasm === 'function'
    ? mod.list_capabilities_wasm()
    : null;
  if (leaveCaps) {
    const list = Array.isArray(leaveCaps) ? leaveCaps : (leaveCaps.capabilities || leaveCaps);
    if (Array.isArray(list) && !list.includes('solid-leave-migrate')) {
      fail('capability solid-leave-migrate not advertised');
    } else if (Array.isArray(list) && !list.includes('solid-qualia-return')) {
      fail('capability solid-qualia-return not advertised');
    } else if (Array.isArray(list)) {
      ok('capabilities solid-leave-migrate + solid-qualia-return advertised');
    }
  }

  if (process.exitCode) {
    console.error('webcivics Solid RDF fixtures FAILED');
  } else {
    console.log('webcivics Solid RDF fixtures PASSED');
    console.log(`PIN_SHA256=${sha}`);
    console.log(`PIN_RAW=${bytes.length}`);
    console.log(`PIN_GZIP=${gz}`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
