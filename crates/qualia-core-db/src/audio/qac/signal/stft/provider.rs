//! Bounded actual Etalon symmetric-Hann, full-complex forward STFT production.
//! No tensor previews, magnitude reduction, zero padding or hidden allocations.
use etalon_signal::audio::{fill_hann_window_into,stft_frame_into};
#[derive(Debug)]pub enum ProviderError{Shape,Frame,ShortInput,Capacity,Limit,Overflow,Nonfinite,Provider(etalon_signal::SignalError)}
#[derive(Clone,Copy,Debug)]pub struct ProviderLimits{pub sample_scalars:u64,pub output_bins:u64,pub frames:u64,pub work:u64}
impl Default for ProviderLimits{fn default()->Self{Self{sample_scalars:4_194_304,output_bins:2_097_152,frames:65_536,work:512<<20}}}
/// Immutable checked layout, row-major [frame][channel][bin][real,imag].
#[derive(Clone,Copy,Debug)]pub struct Plan{sample_frames:usize,channels:usize,fft_size:usize,hop:usize,frames:usize,output_bytes:usize,work:u64}
impl Plan{
 pub fn new(pcm_bytes:usize,channels:u32,fft_size:usize,hop:usize,limits:ProviderLimits)->Result<Self,ProviderError>{
  if channels==0||pcm_bytes%4!=0{return Err(ProviderError::Shape)}
  if !fft_size.is_power_of_two()||!(2..=1024).contains(&fft_size)||hop==0{return Err(ProviderError::Frame)}
  let channels=usize::try_from(channels).map_err(|_|ProviderError::Overflow)?;let scalars=pcm_bytes/4;if scalars%channels!=0{return Err(ProviderError::Shape)}let sample_frames=scalars/channels;
  if sample_frames<fft_size{return Err(ProviderError::ShortInput)}
  let frames=(sample_frames-fft_size)/hop+1;let bins=frames.checked_mul(channels).and_then(|n|n.checked_mul(fft_size)).ok_or(ProviderError::Overflow)?;let output_bytes=bins.checked_mul(8).ok_or(ProviderError::Overflow)?;
  let log=u64::from(fft_size.ilog2());let work=(bins as u64).checked_mul(64+8*log).and_then(|n|n.checked_add(scalars as u64)).and_then(|n|n.checked_add(fft_size as u64*64)).ok_or(ProviderError::Overflow)?;
  if scalars as u64>limits.sample_scalars||bins as u64>limits.output_bins||frames as u64>limits.frames||work>limits.work{return Err(ProviderError::Limit)}
  Ok(Self{sample_frames,channels,fft_size,hop,frames,output_bytes,work})
 }
 pub fn sample_frames(self)->usize{self.sample_frames}
 pub fn channels(self)->usize{self.channels}
 pub fn fft_size(self)->usize{self.fft_size}
 pub fn hop(self)->usize{self.hop}
 pub fn frames(self)->usize{self.frames}
 pub fn output_bytes(self)->usize{self.output_bytes}
 pub fn work_units(self)->u64{self.work}
}
/// Original PCM is exact f32 LE, interleaved time/channel. All source samples,
/// including an untransformed tail, must be finite. Capacity/shape/work/nonfinite
/// input errors precede output mutation. Provider numeric failures may change
/// caller scratch/earlier spectrum rows; only a later admitted document is final.
pub fn transform_into(pcm_le:&[u8],channels:u32,fft_size:usize,hop:usize,mono:&mut[f32],fft:&mut[f32],window:&mut[f32],spectra:&mut[u8],limits:ProviderLimits)->Result<Plan,ProviderError>{
 let plan=Plan::new(pcm_le.len(),channels,fft_size,hop,limits)?;
 if mono.len()<fft_size||fft.len()<fft_size.checked_mul(2).ok_or(ProviderError::Overflow)?||window.len()<fft_size||spectra.len()<plan.output_bytes{return Err(ProviderError::Capacity)}
 for raw in pcm_le.chunks_exact(4){if !f32::from_le_bytes(raw.try_into().unwrap()).is_finite(){return Err(ProviderError::Nonfinite)}}
 fill_hann_window_into(&mut window[..fft_size]).map_err(ProviderError::Provider)?;
 for frame in 0..plan.frames{let start=frame.checked_mul(hop).ok_or(ProviderError::Overflow)?;
  for channel in 0..plan.channels{
   for(i,value)in mono[..fft_size].iter_mut().enumerate(){let sample=(start+i).checked_mul(plan.channels).and_then(|n|n.checked_add(channel)).and_then(|n|n.checked_mul(4)).ok_or(ProviderError::Overflow)?;*value=f32::from_le_bytes(pcm_le[sample..sample+4].try_into().unwrap());}
   stft_frame_into(&mono[..fft_size],&mut fft[..fft_size*2]).map_err(ProviderError::Provider)?;
   let first=(frame*plan.channels+channel)*fft_size*8;
   for(bin,complex)in fft[..fft_size*2].chunks_exact(2).enumerate(){let at=first+bin*8;spectra[at..at+4].copy_from_slice(&complex[0].to_le_bytes());spectra[at+4..at+8].copy_from_slice(&complex[1].to_le_bytes());}
  }
 }
 Ok(plan)
}
