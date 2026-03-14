function test_for
    for i = 1:10
        x = i;
    end
    for j = 1:2:20
        y = j*2;
    end
    parfor k = 1:4
        z = k;
    end
    parfor (m = 1:8, 4)
        w = m;
    end
end
