@examples.json{T Json=Null|Bool(b)|Num(f64)|Str(s)|Arr(Json[])|Obj({s:Json});F main:()->i32!{io}=m(c(parse,"{\"mu\":1}")){Ok(j)=>{c(println,c(stringify,j));0};Er(e)=>{c(println,e);1};};}
