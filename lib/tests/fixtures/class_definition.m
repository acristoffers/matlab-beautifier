classdef MyClass < handle
    properties
        x
        y = 0
    end
    properties (Access=private)
        internal
    end
    events
        Changed
    end
    methods
        function obj = MyClass(x, y)
            obj.x = x;
            obj.y = y;
        end

        function val = getSum(obj)
            val = obj.x+obj.y;
        end

        function set.x(obj, val)
            obj.x = val;
        end
    end
    methods (Static)
        function result = create(x, y)
            result = MyClass(x, y);
        end
    end
end
