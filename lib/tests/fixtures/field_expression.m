function test_fields(obj)
    a = obj.x;
    b = obj.x.y;
    c = obj.method(1, 2);
    d = obj.x.method(3);
    e = obj.method(1).result;
    f = obj.method(1).chain(2);
end
