package main

import (
	"strings"

	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/glamour"
	tea "github.com/charmbracelet/bubbletea"
)

type model struct {
	files           []string
	cursor          int
	renderedContent string
	meta            noteMeta
	rawLines        []string
	notesDir        string
	width           int
	height          int
	rightPanelWidth int
	loading         bool
	renderer        *glamour.TermRenderer
	mode            appMode
	lineCursor      int
	textInput       textinput.Model
}

func newModel(notesDir string) model {
	ti := textinput.New()
	ti.Prompt = ""
	ti.CharLimit = 500

	return model{
		notesDir:  notesDir,
		mode:      ModeNav,
		textInput: ti,
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
		m.rightPanelWidth = m.width - m.width/3 - 4
		m.renderer = newRenderer(m.rightPanelWidth)
		if len(m.files) > 0 && !m.loading && m.renderedContent == "" {
			m.loading = true
			return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
		}
		return m, nil

	case markdownLoadedMsg:
		m.loading = false
		m.renderedContent = msg.content
		m.meta = msg.meta
		m.rawLines = msg.rawLines
		if m.lineCursor >= len(m.rawLines) {
			m.lineCursor = 0
		}
		return m, nil
	}

	if m.mode == ModeEdit {
		return m.updateEdit(msg)
	}

	return m.updateNavExec(msg)
}

func (m model) updateNavExec(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			if m.mode == ModeNav {
				return m, tea.Quit
			}
			m.mode = ModeNav
			return m, nil

		case "esc":
			m.mode = ModeNav
			return m, nil

		case "tab":
			if m.mode == ModeNav {
				m.mode = ModeExec
				if len(m.rawLines) > 0 && m.lineCursor >= len(m.rawLines) {
					m.lineCursor = 0
				}
			} else {
				m.mode = ModeNav
			}
			return m, nil

		case "up", "k":
			if m.mode == ModeNav {
				if m.cursor > 0 && m.renderer != nil {
					m.cursor--
					m.loading = true
					return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
				}
			} else {
				if m.lineCursor > 0 {
					m.lineCursor--
				}
			}
			return m, nil

		case "down", "j":
			if m.mode == ModeNav {
				if m.cursor < len(m.files)-1 && m.renderer != nil {
					m.cursor++
					m.loading = true
					return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
				}
			} else {
				if m.lineCursor < len(m.rawLines)-1 {
					m.lineCursor++
				}
			}
			return m, nil

		case " ":
			if m.mode == ModeExec && len(m.rawLines) > 0 {
				if err := toggleTask(m.notesDir, m.files[m.cursor], m.lineCursor); err == nil {
					m.loading = true
					return m, loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
				}
			}
			return m, nil

		case "e":
			if m.mode == ModeExec && len(m.rawLines) > 0 {
				m.mode = ModeEdit
				m.textInput.SetValue(m.rawLines[m.lineCursor])
				m.textInput.Focus()
				return m, textinput.Blink
			}
			return m, nil
		}
	}

	return m, nil
}

func (m model) updateEdit(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "enter":
			newLine := m.textInput.Value()
			if err := replaceLine(m.notesDir, m.files[m.cursor], m.lineCursor, newLine); err == nil {
				m.loading = true
				cmd := loadNoteCmd(m.notesDir, m.files[m.cursor], m.renderer)
				m.mode = ModeExec
				m.textInput.Blur()
				return m, cmd
			}
			return m, nil

		case "esc":
			m.mode = ModeExec
			m.textInput.Blur()
			return m, nil
		}
	}

	var cmd tea.Cmd
	m.textInput, cmd = m.textInput.Update(msg)
	return m, cmd
}

func (m model) View() string {
	return renderView(m)
}

func (m model) taskCounts() (done, total int) {
	return countTasks(m.rawLines)
}

func (m model) activeFileName() string {
	if len(m.files) == 0 {
		return ""
	}
	return strings.TrimSuffix(m.files[m.cursor], ".md")
}

func (m model) modeString() string {
	switch m.mode {
	case ModeNav:
		return "NAV"
	case ModeExec:
		return "EXEC"
	case ModeEdit:
		return "EDIT"
	default:
		return "???"
	}
}
