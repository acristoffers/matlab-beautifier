function test_lambda
    f = @(x) x^2;
    g = @(x, y) x+y;
    h = @() 42;
    compose = @(f, g) @(x) f(g(x));
end
