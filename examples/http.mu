@examples.http{F fetch:(s)->s!s!{net}=m(c(get,arg0)){Ok(body)=>Ok(body);Er(msg)=>Er(msg);};F main:()->i32=0;}
