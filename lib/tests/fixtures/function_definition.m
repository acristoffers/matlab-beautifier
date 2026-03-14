function simple
    x = 1;
end

function y = with_output(x)
    y = x+1;
end

function [a, b] = multi_output(x, y)
    a = x+1;
    b = y+1;
end

function result = nested(x)
    if x > 0
        result = inner(x);
    else
        result = 0;
    end
end

function y = inner(x)
    y = x*2;
end
