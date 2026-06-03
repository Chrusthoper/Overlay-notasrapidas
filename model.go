package main

import "github.com/charmbracelet/bubbletea"

type model struct {
	files    []string
	cursor   int
	content  string
	notesDir string
	width    int
	height   int
}

func newModel(notesDir string) model {
	return model{
		notesDir: notesDir,
	}
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return m, tea.Quit

		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
				m.loadSelected()
			}

		case "down", "j":
			if m.cursor < len(m.files)-1 {
				m.cursor++
				m.loadSelected()
			}
		}
	}

	return m, nil
}

func (m *model) loadSelected() {
	if len(m.files) == 0 {
		m.content = ""
		return
	}
	content, err := readNote(m.notesDir, m.files[m.cursor])
	if err != nil {
		m.content = "Error al leer el archivo."
		return
	}
	m.content = content
}

func (m model) View() string {
	return renderView(m)
}
