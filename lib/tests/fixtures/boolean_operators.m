function test_boolean(x, y)
    a = x > 0;
    b = x < 0;
    c = x >= 0;
    d = x <= 0;
    e = x == 0;
    f = x ~= 0;
    g = x > 0 && y < 0;
    h = x > 0 || y < 0;
end
