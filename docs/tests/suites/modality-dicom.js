// DICOM split-ingest & anatomy overlay spec — mirrors dicom.rs + dicom_ingest.rs.

import { q_hash, makeQuin } from './primitives.js';

const INLINE_BLOB_TAG = 0b100n << 60n;
const INLINE_MASK     = 0x0FFF_FFFF_FFFF_FFFFn;
const P_HAS_SERIES    = q_hash('q42:hasDicomSeries');
const P_MODALITY      = q_hash('q42:hasModality');
const P_PIXEL_PTR     = q_hash('q42:pixelBlobPointer');
const P_BODY_PART     = q_hash('q42:bodyPartExamined');
const STUDY_CTX       = q_hash('q42:imagingStudy');

function encodeBlobPointer(offset) {
    return (BigInt(offset) & INLINE_MASK) | INLINE_BLOB_TAG;
}

function decodeBlobPointer(field) {
    if ((field & (0b111n << 60n)) !== INLINE_BLOB_TAG) return null;
    return field & INLINE_MASK;
}

function packVolumeMetadata(rows, cols, byteLength) {
    return (BigInt(rows) << 48n) | (BigInt(cols) << 32n) | BigInt(byteLength);
}

function unpackVolumeMetadata(meta) {
    const rows = Number((meta >> 48n) & 0xFFFFn);
    const cols = Number((meta >> 32n) & 0xFFFFn);
    const len  = Number(meta & 0xFFFF_FFFFn);
    return { rows, cols, byteLength: len };
}

function compileDicomQuins(meta, patient, seriesHash, blobOffset, blobLen) {
    const quins = [];
    const vol = packVolumeMetadata(meta.rows, meta.cols, blobLen);
    quins.push(makeQuin(patient, P_HAS_SERIES, seriesHash, STUDY_CTX, vol));
    quins.push(makeQuin(seriesHash, P_MODALITY, q_hash(meta.modality), STUDY_CTX));
    if (meta.bodyPart) {
        quins.push(makeQuin(seriesHash, P_BODY_PART, q_hash(meta.bodyPart), STUDY_CTX));
    }
    quins.push(makeQuin(seriesHash, P_PIXEL_PTR, encodeBlobPointer(blobOffset), STUDY_CTX, vol));
    return quins;
}

function inferOrganFromMeta(meta) {
    const tokens = `${meta.bodyPart} ${meta.seriesDescription} ${meta.studyDescription}`
        .toLowerCase();
    if (/\b(heart|cardiac|coronary|aorta)\b/.test(tokens)) return 'Heart';
    if (/\b(lung|pulmonary|thorax|chest ct)\b/.test(tokens)) return 'Lung';
    if (/\b(brain|cerebral|cranial|head)\b/.test(tokens)) return 'Brain (Allen)';
    return null;
}

export function register(runner) {
    runner.describe('DICOM: split-ingest Quins', () => {

        runner.it('blob pointer uses inline tag 0b100 in bits 60–62', () => {
            const ptr = encodeBlobPointer(4096);
            runner.expect((ptr >> 60n) & 0b111n).toBe(0b100n);
            runner.expect(decodeBlobPointer(ptr)).toBe(4096n);
        });

        runner.it('pack/unpack volume metadata round-trips rows/cols/length', () => {
            const packed = packVolumeMetadata(512, 432, 177652);
            const u = unpackVolumeMetadata(packed);
            runner.expect(u.rows).toBe(512);
            runner.expect(u.cols).toBe(432);
            runner.expect(u.byteLength).toBe(177652);
        });

        runner.it('compile_semantic_quins emits series + modality + pixel pointer', () => {
            const patient = q_hash('did:patient:mr');
            const series = q_hash('1.2.3.4.5');
            const quins = compileDicomQuins(
                { modality: 'MR', bodyPart: 'HEAD', rows: 256, cols: 256, seriesDescription: 'brain mri' },
                patient, series, 8192, 12000,
            );
            runner.expect(quins.length).toBe(4);
            runner.expect(quins[0].predicate).toBe(P_HAS_SERIES);
            runner.expect(quins[1].predicate).toBe(P_MODALITY);
            runner.expect(quins[3].predicate).toBe(P_PIXEL_PTR);
            runner.expect(decodeBlobPointer(quins[3].object)).toBe(8192n);
        });

        runner.it('inferOrganFromMeta maps cardiac CT description to Heart', () => {
            const organ = inferOrganFromMeta({
                bodyPart: 'CHEST',
                seriesDescription: 'Coronary CTA',
                studyDescription: '',
            });
            runner.expect(organ).toBe('Heart');
        });

        runner.it('inferOrganFromMeta maps brain MRI to Brain (Allen)', () => {
            const organ = inferOrganFromMeta({
                bodyPart: '',
                seriesDescription: 'brain MRI',
                studyDescription: 'cranial',
            });
            runner.expect(organ).toBe('Brain (Allen)');
        });

        runner.it('patient is subject of hasDicomSeries, not object of modality', () => {
            const patient = q_hash('did:patient:sep');
            const series = q_hash('series-uid');
            const quins = compileDicomQuins(
                { modality: 'CT', bodyPart: 'CHEST', rows: 2, cols: 2 },
                patient, series, 0, 4,
            );
            runner.expect(quins[0].subject).toBe(patient);
            runner.expect(quins[1].subject).toBe(series);
        });
    });
}

export default register;
