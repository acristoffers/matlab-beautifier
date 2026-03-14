function y = test_args(x, opts)
    arguments
        x (1,:) double {mustBePositive}
        opts.scale (1,1) double = 1.0
    end
    y = x*opts.scale;
end

function test_class_args(obj)
    arguments
        obj.?MyClass
    end
end
