// This is a braindead version of the matches! macro in SimonSapin(?)'s rust-std-candidates repository: https://github.com/SimonSapin/rust-std-candidates#the-matches-macro
macro_rules! matches
{
    ( $x:expr , $( $p:pat )|+ ) => {
        {
            match $x
            {
                $($p)|+ => true,
                _ => false
            }
        }
    };
}
