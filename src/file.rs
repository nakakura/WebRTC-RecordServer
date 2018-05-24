use either::*;
use nom::*;

pub fn e(i: &[u8]) -> &[u8] {
    let x = ex(i);
    match x {
        Left(l) => {
            e(l.1)
        },
        Right(r) => {
            println!("right {:?}", r);
            r
        }
    }
}

pub fn ex(i: &[u8]) -> Either<(&[u8], &[u8]), &[u8]>{
    let x = extract(i);
    if x.is_done() {
        let x = x.unwrap().1;
        Left((x.1, x.2))
    } else if x.is_incomplete() {
        Right(i)
    } else {
        println!("err");
        unreachable!();
    }
}

pub fn extract(i: &[u8]) -> IResult<&[u8], (u16, &[u8], &[u8])> {
    let total_len = i.len();
    do_parse!(i,
                data_len: be_u16
            >>  payload: take!(data_len)
            >>  rest: take!(total_len - data_len as usize - 2)
            >> (
                data_len, payload, rest
            )
        )
}
