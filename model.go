package main

import (
	"github.com/charmbracelet/glamour"
	tea "github.com/charmbracelet/bubbletea"
)

type model struct {
	files           []string
	cursor          int
	renderedContent string
	notesDir        string
	width           int
	height          int
	rightPanelWidth int
	loading         bool
	renderer        *glamour.TermRenderer
}

func newModel(notesDir string, width int) model {
	rightWidth := width - width/3 - 4
	return model{
		notesDir:        notesDir,
		rightPanelWidth: rightWidth,
		renderer:        newRenderer(rightWidth),
	}
}

func (m model) Init() tea.Cmd {
	if len(m.files) > 0 {
		return loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
	}
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.rightPanelWidth = m.width - m.width/3 - 4
		m.renderer = newRenderer(m.rightPanelWidth)
		if m.renderedContent != "" {
			return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
		}
		return m, nil

	case markdownLoadedMsg:
		m.loading = false
		m.renderedContent = msg.content
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return m, tea.Quit

		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
				m.loading = true
				return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
			}

		case "down", "j":
			if m.cursor < len(m.files)-1 {
				m.cursor++
				m.loading = true
				return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
			}
		}
	}

	return m, nil
}

func (m model) View() string {
	return renderView(m)
}
