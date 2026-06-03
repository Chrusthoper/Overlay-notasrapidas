package main

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"
)

func main() {
	notesDir := defaultNotesDir
	if len(os.Args) > 1 {
		notesDir = os.Args[1]
	}

	files, err := scanNotes(notesDir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error al leer la carpeta de notas: %v\n", err)
		os.Exit(1)
	}

	m := newModel(notesDir)
	m.files = files

	p := tea.NewProgram(m, tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
