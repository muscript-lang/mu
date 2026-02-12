# µScript v0.1 Language Tour

## Minimal module

```mu
@demo.hello{E[main];F main:()->i32!{io}={c(print,"hello");0};}
```

## Declarations

```mu
@demo.decls{
E[valx,main];
T Opt[A]=None|Some(A);
V valx:?i32=Some(1);
F main:()->i32=0;
}
```

## Expressions

```mu
@demo.exprs{
V x:i32=v(y:i32=1,v(z:i32=2,z));
V b:i32=i(t,1,0);
V m:i32=m(t){t=>1;f=>0;};
V y:i32=v(fn1=l(a:i32):i32=a,c(fn1,1));
}
```

## Contracts and asserts

```mu
@demo.contracts{
F main:()->i32={
    a(t,"ok");
    ^t;
    _ t;
    0
};
}
```

## Canonical formatting

Formatting is part of the language definition:

```bash
cargo run -- fmt --check .
```

To rewrite source files to canonical form:

```bash
cargo run -- fmt examples
```

## Bytecode run

```bash
cargo run -- build examples/hello.mu -o hello.mub
cargo run -- run hello.mub
```

## Type and effect checking

Pure functions cannot call effectful operations:

```mu
@demo.fx{F main:()->i32={c(print,"x");0};}
```

This is rejected with `E3007` because `print` requires `!{io}`.

## Result and JSON flow

`core.json.parse` returns a `Result` (`Ok` / `Er`) and can be matched directly:

```mu
@demo.json{
T Json=Null|Bool(b)|Num(f64)|Str(s)|Arr(Json[])|Obj({s:Json});
F main:()->i32!{io}=m(c(parse,"{\"mu\":1}")){
Ok(j)=>{c(println,c(stringify,j));0};
Er(e)=>{c(println,e);1};
};
}
```

## HTTP helper shape

`core.http.get` is effect-gated with `!{net}` and returns `s!s`:

```mu
@demo.http{
F fetch:(s)->s!s!{net}=m(c(get,arg0)){
Ok(body)=>Ok(body);
Er(msg)=>Er(msg);
};
F main:()->i32=0;
}
```
