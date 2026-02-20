@eq.json{$[Json];:io=core.io;T #0=Null|Bool(b)|Num(f64)|Str(s)|Arr(#0[])|Obj({s:#0});F main:()->i32!{I}=[m (parse "{\"mu\":1}") {Ok(j) {(println (stringify j));0}} {Er(e) {(println e);1}}];}
