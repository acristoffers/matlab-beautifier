function test_try
    try
        x = dangerous();
    catch e
        x = 0;
    end
    try
        y = risky();
    catch
        y = -1;
    end
end
