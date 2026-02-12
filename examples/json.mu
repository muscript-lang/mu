@examples.json{T Json=Null|Bool(b)|Num(f64)|Str(s)|Arr(Json[])|Obj({s:Json});F main:()->i32={c(parse,"{\"mu\":1}");c(stringify,Str("mu"));0};}
