function y = test_switch(x)
    switch x
        case 1
            y = 'one';
        case {2 3}
            y = 'two or three';
        otherwise
            y = 'other';
    end
end
