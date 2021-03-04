pub type Result<T, LCDER> = core::result::Result<T, Error<LCDER>>;

#[derive(Debug)]
pub enum Error<LCD> {
    // Hw LCD error
    Lcd(LCD),
    // Queue error
    Queue,
    // Buffer
    BufferWrite,
}
