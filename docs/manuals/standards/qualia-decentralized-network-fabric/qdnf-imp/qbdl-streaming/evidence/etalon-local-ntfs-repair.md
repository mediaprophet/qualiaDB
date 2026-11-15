# Local NTFS backing admission repair

The frozen core suite rejected model leases on the actual C: NTFS volume. The opened-handle observation in windows-volume-observation.json returned DeviceType 7, characteristics 0x220020, filesystem NTFS, and a successful FileIsRemoteDeviceInformation response with IsRemote=0 and exactly one byte. The former classifier rejected every characteristic outside its incomplete known-local mask, including this volume's 0x200000 bit. No meaning is assigned to that unknown bit.

The supplying etalon-model-io crate now requires the actual opened-file locality query to complete successfully with exactly one byte equal to zero. It retains disk-device, NTFS, known remote/TS/WebDAV/VSMB rejection, fixed bounded buffers, synchronous handle ownership, and FILE_SHARE_READ exclusion. Unfamiliar unrelated characteristic flags are no longer mistaken for remote backing. Query failure, pending status, short or oversized response, and every nonzero locality byte fail closed.

Primary API references:
- https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryinformationfile
- https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ns-wdm-_file_is_remote_device_information
- https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/616b66d5-b335-4e1c-8f87-b4a55e8d3e4a

Supplier tests: etalon-local-ntfs-repair.log reports 25 passed and one failed. The remaining failure occurs at hard-link fixture creation, before lease acquisition, with Windows access denied. It is not waived; a normal-access test run remains necessary. Actual growth/truncation exclusion, preexisting writable mappings, cancellation, cleanup, content checking and zero-allocation borrow tests passed. No GPU behavior changes.

The consuming workspace uses one explicit Cargo git-source patch to ../etalon/crates/etalon-model-io so both direct and transitive users share the repaired supplier. Other Etalon crates remain pinned. This local multi-repository integration is not a published dependency revision and requires both workspace repositories. Core integration tests and lockfile resolution are pending.
