use crate::error::FontError;

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], FontError> {
        let end = self.pos.checked_add(n).ok_or(FontError::UnexpectedEof)?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or(FontError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, FontError> {
        Ok(self.take(1)?[0])
    }

    pub fn i8(&mut self) -> Result<i8, FontError> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16, FontError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16, FontError> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32, FontError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn tag(&mut self) -> Result<[u8; 4], FontError> {
        let b = self.take(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }
}
