use super::*;
struct GridDomain {
    allow_entry: bool,
}
impl DomainRegistry for GridDomain {
    fn validate_grid_frame(&self, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
        // Exercise public budgeted lookup; real domain implementations resolve extensions.
        let mut found = false;
        for block in ctx.blocks() {
            if block?.id() == frame.id() {
                found = true;
            }
        }
        if found {
            Ok(())
        } else {
            Err(Error::Reference)
        }
    }
    fn validate_spatial_entry(
        &self,
        _: Entry<'_>,
        _: Block<'_>,
        _: &Context<'_, '_>,
    ) -> Result<(), Error> {
        if self.allow_entry {
            Ok(())
        } else {
            Err(Error::Frame)
        }
    }
}
#[test]
fn family_order_requires_both_frame_and_entry_admission() {
    let mut b = good();
    b[1].fields.retain(|f| f.key != 7);
    let encoded = bytes(&b, &[row()]);
    assert_eq!(
        result_with(&encoded, &SharedOnly, Limits::default()),
        Err(Error::UnsupportedDomain)
    );
    assert_eq!(
        result_with(
            &encoded,
            &GridDomain { allow_entry: false },
            Limits::default()
        ),
        Err(Error::Frame)
    );
    assert_eq!(
        result_with(
            &encoded,
            &GridDomain { allow_entry: true },
            Limits::default()
        ),
        Ok(())
    );
    b[1].fields.retain(|f| f.key != 5);
    assert_eq!(
        result_with(
            &bytes(&b, &[row()]),
            &GridDomain { allow_entry: true },
            Limits::default()
        ),
        Err(Error::Frame)
    );
}
#[test]
fn domain_cannot_bypass_shared_spatial_bounds_or_critical_fields() {
    let registry = GridDomain { allow_entry: true };
    assert_eq!(
        result_with(
            &bytes(
                &good(),
                &[EntryInput {
                    spatial_key: 24,
                    ..row()
                }]
            ),
            &registry,
            Limits::default()
        ),
        Err(Error::Frame)
    );
    assert_eq!(
        result_with(
            &bytes(&good(), &[row()]),
            &GridDomain { allow_entry: false },
            Limits::default()
        ),
        Err(Error::Frame)
    );
    let mut b = good();
    b[1].fields.retain(|f| f.key != 7);
    change(&mut b, 2, 5).critical = false;
    assert_eq!(
        result_with(&bytes(&b, &[row()]), &registry, Limits::default()),
        Err(Error::RequiredCritical)
    );
    change(&mut b, 2, 5).critical = true;
    change(&mut b, 2, 5).value = [1.0f64, 0.0, 1.0]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    assert_eq!(
        result_with(&bytes(&b, &[row()]), &registry, Limits::default()),
        Err(Error::Frame)
    );
}
#[test]
fn metadata_iterator_charges_each_traversed_block() {
    let encoded = bytes(&good(), &[row()]);
    let view =
        StructuralView::open(&encoded, Family::QacV1Candidate, StorageLimits::default()).unwrap();
    let ctx = Context {
        view: &view,
        work: std::cell::Cell::new(1),
    };
    let mut iter = ctx.blocks();
    assert_eq!(iter.next().unwrap().unwrap().id(), 1);
    assert!(matches!(iter.next(), Some(Err(Error::Limit))));
    assert_eq!(ctx.remaining_work(), 0);
}
