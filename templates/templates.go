package templates

import _ "embed"

//go:embed index.html
var IndexHTML string

//go:embed app.css
var AppCSS string

//go:embed app.js
var AppJS string
