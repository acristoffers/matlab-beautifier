% Top-level standalone comment

function test_comments
    % This is a function comment
    x = 1; % inline comment
           % % Section header
    y = 2;
    %{
      Block comment
      second line
    %}
    % Multi-line
    % comment block
    z = 3;
    %#ok lint suppression
end
