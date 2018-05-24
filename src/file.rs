use nom::*;

pub fn parse<'a>(mut v: Vec<&'a [u8]>, i: &'a [u8]) -> (Vec<&'a [u8]>, &'a [u8]) {
    let x = extract(i);
    if x.is_done() {
        let x = x.unwrap().1;
        v.push(x.1);
        parse(v, x.2)
    } else if x.is_incomplete() {
        (v, i)
    } else {
        println!("err");
        unreachable!();
    }
}

fn extract(i: &[u8]) -> IResult<&[u8], (u16, &[u8], &[u8])> {
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

