use std::borrow::Cow;
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::IpAddr;
use std::path::Path;
use std::sync::OnceLock;

use crate::header::{
    HEADER_INFO_LENGTH, Header, IpVersion, VECTOR_INDEX_COLS, VECTOR_INDEX_LENGTH,
    VECTOR_INDEX_SIZE,
};
use tracing::{debug, trace, warn};

use crate::error::{Ip2RegionError, Result};
use crate::ip_value::{CompareExt, IpValueExt};

pub struct Searcher {
    pub filepath: String,
    pub cache_policy: CachePolicy,
    pub header: Header,
    vector_cache: OnceLock<Vec<u8>>,
    full_cache: OnceLock<Vec<u8>>,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum CachePolicy {
    NoCache,
    VectorIndex,
    FullMemory,
}

impl Searcher {
    pub fn new(filepath: String, cache_policy: CachePolicy) -> Result<Self> {
        let mut file = File::open(Path::new(&filepath))?;
        let mut buf = [0; HEADER_INFO_LENGTH];
        file.read_exact(&mut buf)?;

        let header = Header::try_from(&buf)?;
        debug!(?header, "Load xdb file with header");

        Ok(Self {
            filepath,
            cache_policy,
            header,
            vector_cache: OnceLock::new(),
            full_cache: OnceLock::new(),
        })
    }

    pub fn search<T>(&self, ip: T) -> Result<String>
    where
        T: IpValueExt + Display,
    {
        let ip = ip.to_ipaddr()?;

        let (il0, il1) = match (ip, self.header.ip_version()) {
            (IpAddr::V6(ip), IpVersion::V6) => (ip.octets()[0], ip.octets()[1]),
            (IpAddr::V4(ip), IpVersion::V4) => (ip.octets()[0], ip.octets()[1]),
            (_, IpVersion::V4) => return Err(Ip2RegionError::OnlyIPv4Version),
            (_, IpVersion::V6) => return Err(Ip2RegionError::OnlyIPv6Version),
        };

        let start_point = VECTOR_INDEX_SIZE * ((il0 as usize) * VECTOR_INDEX_COLS + (il1 as usize));
        let vector_index = self.vector_index()?;
        let start_ptr =
            u32::from_le_bytes(vector_index[start_point..start_point + 4].try_into()?) as usize;
        let end_ptr =
            u32::from_le_bytes(vector_index[start_point + 4..start_point + 8].try_into()?) as usize;

        // @Note: ptr validate, zero ptr means source data missing
        // so we could just stop here and return an empty string.
        if start_ptr == 0 || end_ptr == 0 {
            return Ok(String::new());
        }

        // Binary search the segment index to get the region
        let segment_index_size = self.header.segment_index_size();
        let ip_bytes_len = self.header.ip_bytes_len();
        let ip_end_offset = ip_bytes_len * 2;

        let mut left: usize = 0;
        let mut right: usize = (end_ptr - start_ptr) / segment_index_size;

        while left <= right {
            let mid = (left + right) >> 1;
            let offset = start_ptr + mid * segment_index_size;
            let buffer_ip_value = self.read_buf(offset, segment_index_size)?;
            if ip.ip_lt(Cow::Borrowed(&buffer_ip_value[0..ip_bytes_len])) {
                let Some(m) = mid.checked_sub(1) else { break };
                right = m;
            } else if ip.ip_gt(Cow::Borrowed(&buffer_ip_value[ip_bytes_len..ip_end_offset])) {
                left = mid + 1;
            } else {
                let data_length = u16::from_le_bytes([
                    buffer_ip_value[ip_end_offset],
                    buffer_ip_value[ip_end_offset + 1],
                ]);
                let data_offset = u32::from_le_bytes(
                    buffer_ip_value[ip_end_offset + 2..ip_end_offset + 6].try_into()?,
                );
                let result = String::from_utf8(
                    self.read_buf(data_offset as usize, data_length as usize)?
                        .to_vec(),
                )?;
                return Ok(result);
            }
        }
        // From xdb 3.0 version no matched IP result change to empty string,
        // so users should check string is empty.
        //
        // Err(Ip2RegionError::NoMatchedIP)
        Ok(String::new())
    }

    pub fn vector_index(&self) -> Result<Cow<'_, [u8]>> {
        if self.cache_policy.eq(&CachePolicy::NoCache) {
            return self.read_buf(HEADER_INFO_LENGTH, VECTOR_INDEX_LENGTH);
        }

        match self.vector_cache.get() {
            None => {
                debug!("Load vector index cache");
                let data = self
                    .read_buf(HEADER_INFO_LENGTH, VECTOR_INDEX_LENGTH)?
                    .to_vec();
                let _ = self
                    .vector_cache
                    .set(data)
                    .inspect_err(|_| warn!("Vector index cache already initialized"));

                // Safety: vector cache checked and set for empty before
                let cache = self.vector_cache.get().unwrap();
                Ok(Cow::Borrowed(cache))
            },
            Some(cache) => Ok(Cow::Borrowed(cache)),
        }
    }

    pub fn read_buf(&self, offset: usize, size: usize) -> Result<Cow<'_, [u8]>> {
        trace!(offset, size = size, "Read buffer");
        if self.cache_policy.ne(&CachePolicy::FullMemory) {
            debug!(filepath=?self.filepath, offset=offset, size=size, "Read buf without cache");
            let mut file = File::open(&self.filepath)?;
            file.seek(SeekFrom::Start(offset as u64))?;

            let mut buf = vec![0u8; size];
            file.take(size as u64).read_exact(&mut buf)?;
            return Ok(Cow::from(buf));
        }

        match self.full_cache.get() {
            None => {
                debug!(filepath=?self.filepath, "Load full cache");
                let mut file = File::open(&self.filepath)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                let _ = self
                    .full_cache
                    .set(buf)
                    .inspect_err(|_| warn!("Full cache already initialized"));

                // Safety: FULL_CACHE checked and set for empty before
                let cache = self.full_cache.get().unwrap();
                Ok(Cow::from(&cache[offset..offset + size]))
            },
            Some(cache) => {
                let data = Cow::from(&cache[offset..offset + size]);
                Ok(data)
            },
        }
    }
}
