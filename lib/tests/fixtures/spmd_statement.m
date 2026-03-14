function test_spmd
    spmd
        x = labindex;
    end
    spmd(4)
        y = labindex*2;
    end
end
