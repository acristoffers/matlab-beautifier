function test_calls
    a = foo();
    b = foo(1, 2, 3);
    c = foo(x, y, z);
    d = foo(a+b, c*d);
    e = foo(1:10);
    f = foo(:);
    g = bar{1};
    h = bar{1, 2};
    i = foo(bar(1), baz(2));
end
