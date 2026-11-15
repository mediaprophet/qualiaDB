# Network integration readiness — runtime, bridge and endpoint regressions

2026-11-13. All listed sources frozen for root integration build/tests. Worker started NO builds, helpers/background processes or stack-environment changes. Root owns all execution/qualification/programme status. No unrelated Cargo/geometry/model/crypto-parent edits.

Applied with explicit root approval:7 runtime/incarnation parent files;5 channel/resource bridge parent files;25 new runtime/incarnation children;5 new bridge children. Existing transition/stage bodies remain one canonical implementation, constructor/close factored into counted/phase children. Core resource/rights principles and profile/32handle ceilings unchanged. All touched files were scoped-rustfmt2021 successfully. Root corrected direct-source memory test's private page-field access to existing with_candidate_page borrow; no visibility widening/fake owner. Pure runtime tests use actual canonical ledger admission; genuine endpoint integration runs only under original full-core managed fixture. No harness stubs/Cargo cfg bypass added.

Root's OLD-exe endpoint selector exposed stack overflow;16MiB diagnostic25PASS/3FAIL was diagnostic only. Three specific errors were overwritten by with_current's unconditional post-core check after inner guard cleared keys. Source fixed: preserve original fatal error; final fresh check remains for success/WouldBlock; actual output erase guards unchanged and caught errors remain terminal. No assertions weakened.

Root separately approved production IPC fix: former inline2*32*8192 queue payload plus metadata caused ~512KiB constructor/value moves. Private PairState now holds2 fixed boxed arrays, built COLD with explicit try_reserve_exact(32), bounded fill and boxed-slice/array conversion. No unsafe or cap increase; actual PairState header compile-time<=128B. Public ipc_pair and hot metadata/order/queue semantics preserved. Caller bearer backing remains separately admitted as before; no claimed global byte authority. New actual queue regression fills32, rejects33, verifies FIFO/observed metadata, measures raw no-allocation hot path with real positive control. Cold workspace fixture admission precedes safe Box::write construction per workspace; huge tuple removed, no zero-stack/zero-copy claim. Actual hot channel counter window remains unchanged.

Awaiting root NEW-exe DEFAULT-stack tests; do not call diagnostic stack setting qualification. Last root-reported prior wave: core5809PASS, bundle::stream148PASS, mesh16PASS,current_guard3PASS. These precede this final runtime/bridge/IPC source wave. Runtime scoped tests13 and bridge tests5 authored; new IPC test1 authored, no worker test execution. Focused root retry86061 uses actual production sources. Raw pointer/privacy fix in that harness test was root-owned.

Remaining required service work: actual carrier/assembly wrapper with outer-final acceptance abort; bounded native storage/source job handoff outside short parent guards; actual native OS bearer; actual authority-value incarnation follow-up under current trusted host no-replacement assumption; remote Share/Sentinel/contact/consent integration. No completed native transfer/G01/durability/full-programme claim.

## Exact current relevant source fingerprints (relative core src, SHA256)

net/peer/runtime/incarnation.rs 94a53b5d8e789601014a6bbcd41cbba1900cac6c0dfdbf0b6b7af08c2f88aba5
net/peer/runtime/ledger/incarnation.rs d685a1460dd09c23675a8f5453f87a596f46c9d2004e97c60bf7e66d943728e0
net/peer/runtime/leases/incarnation.rs 533c81d2705b6e70d7fb6556f6f382f396c84aa878826e17b4e42639b42bd0d4
net/peer/runtime/cancel/incarnation.rs d43287d269953679ba18a8f20910a901593c6f94a975b21d474ddff14cede02b
net/peer/runtime/scheduler/incarnation.rs f05ec1fa5164447d33f1441446a7ec7ca90c15ec66d67a82e87447b13f6ac7f1
bundle/stream/runtime/state/data.rs 06d91891e90fb22184c393ea192e7f2f53d5c89a53189a8fc93c40df8bca0f91
bundle/stream/runtime/state/restore.rs 55d9dc3986ba25b21ed4d21c5fd0660793014f0e45bee83584137df6274a176b
net/peer/runtime/ledger/inspect.rs e33e6fc4946814d42ce821428c526134e606bdf570933bb7a91b884db20888f7
net/peer/runtime/leases/inspect.rs cf730a428a568f44838f2c79a473d08d9714f4a4c728d624343332798ca880b3
net/peer/runtime/cancel/inspect.rs 26abc3cf9b291f024b98395505842b899208dc9ccdd7b4ca3ec284935a506bfe
net/peer/runtime/scheduler/inspect.rs 2bd0e8f86d467446f0568ce7ab5b82c7673059e8eecf3ccfc4daf1f7288d3d92
bundle/stream/runtime/state/parents.rs 5bf5952189d7ef018fb5a527ac1b047e5d0e176a43cae21ef498ca1fb4ba165b
bundle/stream/runtime/state/loans.rs 14bf18797c65d5848c94c19f559bc899a7788f7aed06b88c64f6fb80b66993c0
bundle/stream/runtime/state/mod.rs 340b2b928ec5bec101194a672c71652145d831ae411d6ac3e991204daf1de5c1
bundle/stream/runtime/owner/counted.rs 6947e4290731f542334318422c3597b05f9499dff424a3f89359ee807729054e
bundle/stream/runtime/owner/close.rs df6d01bfe7cda2d35aeb278dbccee7c9e9fb3d7c9687b7407d684508ed786077
bundle/stream/runtime/state/access.rs ebaf620aec6dec8044f31255d9b96764889ecaf2b0ee4b686d0768f09046f517
bundle/stream/runtime/state/preflight.rs 319722f4ebc73e90e8d006a47bb8195f82979f90d9927327b1fdba31d68f6ed9
bundle/stream/runtime/state/scope.rs 1a997d462a9c6a6cb3f0f3a4969b11dee6e81608728810d3d06a4b6c202a4bf4
bundle/stream/runtime/state/close.rs 435a63a04186609d3f0c5b4e1de351faebb50639cb519bc8a7739fbd52668d02
bundle/stream/runtime/state/tests.rs 6d61671495a7e54a559c34999beeb8b26cd18ab2bd9911a40518e993ab7d7654
bundle/stream/runtime/state/tests/fixtures.rs f570b61474107e7374ad0c04a845786c5c5dfeb787cfd8e0de02d68ee445ad87
bundle/stream/runtime/state/tests/lifecycle.rs ee43bc6676bd10ab4f29c13d4c2ab7eed5c5f38af5a9906cac08d75310a31125
bundle/stream/runtime/state/tests/identity.rs 229504283e4ef33f4cff6b876b3f9f607c90ba9309a82c05d8304db646ab4f6f
bundle/stream/runtime/state/tests/memory.rs 88e5f93191d1ad18c196ddd9930ef4a027e0e115ad9e4e319994f7051ecfec4f
net/qdnf/session/handshake/endpoint/resource/identity.rs 0f7402bdd8231c6b82d41891e40ff19ebdd0d11f382893b53ca5df29d32ce84c
net/qdnf/session/handshake/endpoint/resource/retained/identity.rs 34654f85d80d88481360afdcc0c7946bcb74a616195ae95aafae6ffa007084cb
net/qdnf/session/handshake/endpoint/established/protected_channel/runtime_bridge.rs fb8fb4438aab8cc3fdda0483f0fc028b7f2df29b353327aefb4cc380cb380b87
net/qdnf/session/handshake/endpoint/resource/identity_tests.rs 3cb53985c11a702bef636f1651d4748c8a13d3f7e021d99e25599a31b0545203
net/qdnf/session/handshake/endpoint/established/protected_channel/endpoint_runtime_tests.rs b7c038151951edb785fdaaa9083f0977d1ec293adf3606340dfafca67cd95632
net/peer/runtime/mod.rs 335da6b56088fad623e49138502211ed8168567dd85bbd672215778cb0f427e2
net/peer/runtime/ledger.rs 67d2f404ed1a3ff934cd20403852d5d3b08a867bc5e019f7223bf09d46541b3f
net/peer/runtime/leases.rs b44e7eb4031001fb763cd06d6d2a6bed1024fe32e380d974e1ba829b212d85ba
net/peer/runtime/cancel.rs 6adca12c9f38ee33e692bc01af67b433fd8a773d66e165ce89f0c42838b5ef2a
net/peer/runtime/scheduler.rs bf9cb5cae5d443906278c06bd5b01956e2a035c3c0de84b5757d5460b69cd278
bundle/stream/runtime/mod.rs 1edb99c39228e71f5c40aa534759aa3dbf7d1e34146e70c2088d440890ed68ef
bundle/stream/runtime/owner.rs 198e548b87b565fd1659fc7f6add01c0b88c7ab46692caa80ddbaab68bde935b
net/qdnf/session/handshake/endpoint/resource.rs 8c6489be3bc2f72f8a0cbd680623736c4eccf25d802e6a928f87cd1cbdb1f409
net/qdnf/session/handshake/endpoint/resource/retained.rs 4a712c4bd4227e83b93d114c6575d56f025b7ff34db7c246965866c62ea1c639
net/qdnf/session/handshake/endpoint/established/protected_channel/mod.rs 97fa481554319a2c302c4175efc4f69f1f67e6a04079d68286a5e6264988ce63
net/qdnf/session/handshake/endpoint/established/protected_channel/scope.rs 3270e5027ffa265d09804dd9c2e4f78837eca27da4d676dbcf6325c9af8dadb7
net/qdnf/session/handshake/endpoint/established/protected_channel/endpoint_tests.rs 2101c8c8c6b18f78ba0bcfee1a1a2d84fc21079d7ea7cc4862673bcf880046ce
net/qdnf/bearer/ipc.rs 7ef5f69c0e549fb1a8ac54cf7ce00eaa81b9b26059debe17661a01597880a387
net/qdnf/session/handshake/endpoint/established/protected_channel/endpoint_fixtures.rs b217dc1522e84e76713c586e359f4bc943d5b703d9204b12bc25f9c0e0da5ab5
