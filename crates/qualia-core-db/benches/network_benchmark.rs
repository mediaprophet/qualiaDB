//! QualiaDB Native Zero-Heap DNS & Network Benchmark Suite
//! ========================================================
//! Reproducible Criterion benchmarks measuring throughput and latency of
//! QualiaDB's zero-heap network primitives:
//!
//! 1. Wire Query Builder (RFC 1035 / RFC 3597 packet formatting)
//! 2. Wire Response Parser (Record parsing & label pointer decompression)
//! 3. Q42 Super-Quin DNS Codec (A, lossless 128-bit AAAA, MX, SRV, SDN Front-Door)
//! 4. Zero-Heap DnsCacheRing (Insertion, circular wrap eviction, point & batch lookup)
//! 5. SDN Front-Door Parser (_qdp.<domain> TXT key-value extraction)

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use qualia_core_db::net::dns::*;
use qualia_core_db::{q_hash, NQuin, PermissiveRoutingLane};
use std::hint::black_box;

// ─── 1. Wire Query Packet Builder ───────────────────────────────────────────
fn bench_wire_query_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire_query_builder");
    group.throughput(Throughput::Elements(1));

    let mut buf = [0u8; 512];

    group.bench_function("query_a_record", |b| {
        b.iter(|| {
            let len = build_query_packet(
                black_box(0x1337),
                black_box("api.webizen.network"),
                black_box(DnsType::A),
                black_box(true),
                black_box(&mut buf),
            )
            .unwrap();
            black_box(len);
        })
    });

    group.bench_function("query_aaaa_ipv6_record", |b| {
        b.iter(|| {
            let len = build_query_packet(
                black_box(0x2442),
                black_box("mesh.qualia.internal"),
                black_box(DnsType::AAAA),
                black_box(true),
                black_box(&mut buf),
            )
            .unwrap();
            black_box(len);
        })
    });

    group.bench_function("query_https_rfc9460_record", |b| {
        b.iter(|| {
            let len = build_query_packet(
                black_box(0x4242),
                black_box("secure.commons.qualia.net"),
                black_box(DnsType::HTTPS),
                black_box(true),
                black_box(&mut buf),
            )
            .unwrap();
            black_box(len);
        })
    });

    group.finish();
}

// ─── 2. Wire Response Packet Parser ─────────────────────────────────────────
fn build_synthetic_multi_record_packet() -> ([u8; 512], usize) {
    let mut packet = [0u8; 512];
    let qlen =
        build_query_packet(0x1337, "gateway.webizen.net", DnsType::A, true, &mut packet).unwrap();

    let mut resp_header = DnsHeader::new_query(0x1337, true);
    resp_header.flags |= wire::FLAG_QR_RESPONSE | wire::FLAG_AA_AUTHORITATIVE;
    resp_header.ancount = 3;
    resp_header.encode(&mut packet[0..12]).unwrap();

    let mut offset = qlen;

    // Record 1: A record (192.168.1.100)
    packet[offset] = 0xC0; // Pointer to name at offset 12
    packet[offset + 1] = 0x0C;
    offset += 2;
    packet[offset..offset + 2].copy_from_slice(&1u16.to_be_bytes()); // Type A
    packet[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes()); // Class IN
    packet[offset + 4..offset + 8].copy_from_slice(&300u32.to_be_bytes()); // TTL
    packet[offset + 8..offset + 10].copy_from_slice(&4u16.to_be_bytes()); // RDLength
    packet[offset + 10..offset + 14].copy_from_slice(&[192, 168, 1, 100]);
    offset += 14;

    // Record 2: AAAA record (2606:4700:4700::1111)
    packet[offset] = 0xC0;
    packet[offset + 1] = 0x0C;
    offset += 2;
    packet[offset..offset + 2].copy_from_slice(&28u16.to_be_bytes()); // Type AAAA
    packet[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes());
    packet[offset + 4..offset + 8].copy_from_slice(&600u32.to_be_bytes());
    packet[offset + 8..offset + 10].copy_from_slice(&16u16.to_be_bytes());
    let v6: [u8; 16] = [
        0x26, 0x06, 0x47, 0x00, 0x47, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0x11, 0x11,
    ];
    packet[offset + 10..offset + 26].copy_from_slice(&v6);
    offset += 26;

    // Record 3: TXT record
    packet[offset] = 0xC0;
    packet[offset + 1] = 0x0C;
    offset += 2;
    packet[offset..offset + 2].copy_from_slice(&16u16.to_be_bytes()); // Type TXT
    packet[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes());
    packet[offset + 4..offset + 8].copy_from_slice(&3600u32.to_be_bytes());
    let txt_payload = b"\x14qdp:agentType=device";
    packet[offset + 8..offset + 10].copy_from_slice(&(txt_payload.len() as u16).to_be_bytes());
    packet[offset + 10..offset + 10 + txt_payload.len()].copy_from_slice(txt_payload);
    offset += 10 + txt_payload.len();

    (packet, offset)
}

fn bench_wire_response_parser(c: &mut Criterion) {
    let (packet, len) = build_synthetic_multi_record_packet();
    let resolver = QualiaDnsResolver::new(ResolverConfig::default());
    let mut out_quins = [NQuin::default(); 8];

    let mut group = c.benchmark_group("wire_response_parser");
    group.throughput(Throughput::Elements(3)); // 3 records per packet

    group.bench_function("parse_multi_record_packet_zero_heap", |b| {
        b.iter(|| {
            let n = resolver
                .parse_response_into(
                    black_box(&packet[..len]),
                    black_box(0x1337),
                    black_box(&mut out_quins),
                )
                .unwrap();
            black_box(n);
        })
    });

    group.finish();
}

// ─── 3. Q42 Super-Quin DNS Codec ───────────────────────────────────────────
fn bench_quin_record_codec(c: &mut Criterion) {
    let mut group = c.benchmark_group("quin_record_codec");
    group.throughput(Throughput::Elements(1));

    let ipv4 = [10, 0, 0, 42];
    let ipv6: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let lane = PermissiveRoutingLane::EnforceBilateralMicroCommons;

    group.bench_function("encode_a_record_ipv4", |b| {
        b.iter(|| {
            let q = encode_a_record(
                black_box("node.qualia.net"),
                black_box(ipv4),
                black_box(300),
                black_box(lane),
            );
            black_box(q);
        })
    });

    let quin_a = encode_a_record("node.qualia.net", ipv4, 300, lane);
    group.bench_function("decode_a_record_ipv4", |b| {
        b.iter(|| {
            let res = decode_a_record(black_box(&quin_a));
            black_box(res);
        })
    });

    group.bench_function("encode_aaaa_record_lossless_ipv6", |b| {
        b.iter(|| {
            let q = encode_aaaa_record(
                black_box("v6.qualia.net"),
                black_box(ipv6),
                black_box(7200),
                black_box(lane),
            );
            black_box(q);
        })
    });

    let quin_aaaa = encode_aaaa_record("v6.qualia.net", ipv6, 7200, lane);
    group.bench_function("decode_aaaa_record_lossless_ipv6", |b| {
        b.iter(|| {
            let res = decode_aaaa_record(black_box(&quin_aaaa));
            black_box(res);
        })
    });

    group.bench_function("encode_sdn_front_door", |b| {
        let did_ptr = DID_Q42_FLAG | 0x9876_5432_10FE_DCBA;
        b.iter(|| {
            let q = encode_sdn_front_door(
                black_box("did.webizen.net"),
                black_box(did_ptr),
                black_box(600),
                black_box(lane),
            );
            black_box(q);
        })
    });

    group.finish();
}

// ─── 4. Zero-Heap DnsCacheRing ──────────────────────────────────────────────
fn bench_dns_cache_ring(c: &mut Criterion) {
    let mut group = c.benchmark_group("dns_cache_ring_256");
    group.throughput(Throughput::Elements(1));

    let mut ring: DnsCacheRing<256> = DnsCacheRing::new();
    let sample_quin = encode_a_record(
        "bench.qualia.net",
        [127, 0, 0, 1],
        300,
        PermissiveRoutingLane::PassthroughStandard,
    );

    // Warm up the ring
    for i in 0..256 {
        ring.insert(sample_quin, i);
    }

    group.bench_function("insert_with_fifo_wrap", |b| {
        let mut clock = 1000u32;
        b.iter(|| {
            ring.insert(black_box(sample_quin), black_box(clock));
            clock = clock.wrapping_add(1);
        })
    });

    let target_hash = q_hash("bench.qualia.net");
    group.bench_function("point_lookup_hit", |b| {
        b.iter(|| {
            let res = ring.lookup(black_box(target_hash), black_box(OP_DNS_A));
            black_box(res);
        })
    });

    let mut out_batch = [NQuin::default(); 4];
    group.bench_function("batch_lookup_all_zero_heap", |b| {
        b.iter(|| {
            let n = ring.lookup_all(black_box(target_hash), black_box(&mut out_batch));
            black_box(n);
        })
    });

    group.finish();
}

// ─── 5. SDN Front-Door Key-Value Parser ─────────────────────────────────────
fn bench_sdn_front_door_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("sdn_front_door_parser");
    group.throughput(Throughput::Elements(1));

    let sample_txt = "qdp:signer=did:q42:7f8e9d0a1b2c; qdp:agentType=device; qdp:wireguard=wg-pubkey-base64-here; qdp:overlay=mesh.qualia.net; qdp:nym=nym-address-here";

    group.bench_function("parse_front_door_txt_zero_heap", |b| {
        b.iter(|| {
            let view =
                parse_front_door_txt(black_box("agent.alice.net"), black_box(sample_txt)).unwrap();
            black_box(view);
        })
    });

    let ns_host = "ns1.7f8e9d0a1b2c3d4e5f.webizen.network";
    group.bench_function("parse_ns_encoded_did_zero_heap", |b| {
        b.iter(|| {
            let ptr = parse_ns_encoded_did(black_box(ns_host)).unwrap();
            black_box(ptr);
        })
    });

    group.finish();
}

// ─── 6. DNS Record Ontology Registry ───────────────────────────────────────
fn bench_dns_ontology(c: &mut Criterion) {
    let mut group = c.benchmark_group("dns_ontology_registry");
    group.throughput(Throughput::Elements(1));

    group.bench_function("lookup_by_code_https", |b| {
        b.iter(|| {
            let desc = lookup_by_code(black_box(65)).unwrap();
            black_box(desc);
        })
    });

    group.bench_function("lookup_by_name_aaaa", |b| {
        b.iter(|| {
            let desc = lookup_by_name(black_box("AAAA")).unwrap();
            black_box(desc);
        })
    });

    group.finish();
}

// ─── 7. Lossless Arbitrary-Length RDATA Chunker ─────────────────────────────
fn bench_lossless_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("lossless_rdata_chunking");
    group.throughput(Throughput::Bytes(64));

    let dnskey_64b = [0x5Au8; 64];
    let mut quins = [NQuin::default(); 4];

    group.bench_function("encode_64b_dnskey_into_4_quins", |b| {
        b.iter(|| {
            let n = encode_lossless_rdata_chunks(
                black_box("mesh.qualia.net"),
                black_box(DnsType::DNSKEY),
                black_box(3600),
                black_box(&dnskey_64b),
                black_box(PermissiveRoutingLane::EnforceBilateralMicroCommons),
                black_box(&mut quins),
            )
            .unwrap();
            black_box(n);
        })
    });

    let num_chunks = encode_lossless_rdata_chunks(
        "mesh.qualia.net",
        DnsType::DNSKEY,
        3600,
        &dnskey_64b,
        PermissiveRoutingLane::EnforceBilateralMicroCommons,
        &mut quins,
    )
    .unwrap();

    let mut out_payload = [0u8; 64];
    group.bench_function("decode_64b_dnskey_from_4_quins", |b| {
        b.iter(|| {
            let n = decode_lossless_rdata_chunks(
                black_box(&quins[..num_chunks]),
                black_box(DnsType::DNSKEY),
                black_box(&mut out_payload),
            )
            .unwrap();
            black_box(n);
        })
    });

    group.finish();
}

criterion_group!(
    network_benches,
    bench_wire_query_builder,
    bench_wire_response_parser,
    bench_quin_record_codec,
    bench_dns_cache_ring,
    bench_sdn_front_door_parser,
    bench_dns_ontology,
    bench_lossless_chunking,
);

criterion_main!(network_benches);
