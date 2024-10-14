function init(window)

end

function frame(ui)
    ui.label("Test", 0)
    ui.header("test", function(hd)
        hd.label("Health", 0)
    end)
end