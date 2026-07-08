import { defineComponent, h, reactive, ref } from 'vue'
import { apiGet, apiPost, type EventPage, type SessionEvent } from '../api'
import { ElCheckbox, ElMessage, ElMessageBox } from 'element-plus'

const DeleteEventConfirmContent = defineComponent({
  props: {
    eventIndex: {
      type: Number,
      required: true,
    },
  },
  emits: ['change'],
  setup(props, { emit }) {
    const deleteAfter = ref(false)
    return () =>
      h('div', { class: 'delete-event-confirm' }, [
        h('p', `删除节点 #${props.eventIndex} 会直接重写该会话 JSONL，且不会自动备份。`),
        h(
          ElCheckbox,
          {
            modelValue: deleteAfter.value,
            'onUpdate:modelValue': (value: string | number | boolean) => {
              deleteAfter.value = Boolean(value)
              emit('change', deleteAfter.value)
            },
          },
          () => '包括之后的所有条目',
        ),
      ])
  },
})

export function useThreadEvents(selectedId: ReturnType<typeof ref<string>>) {
  const events = ref<SessionEvent[]>([])
  const eventsTotal = ref(0)
  const loadingEvents = ref(false)
  const expandedEvents = ref(new Set<number>())

  const eventFilters = reactive({
    event_type: '',
    payload_type: '',
    role: '',
    q: '',
    limit: 80,
  })

  async function loadEvents(reset = false) {
    if (!selectedId.value) return
    loadingEvents.value = true
    try {
      const page = await apiGet<EventPage>(`/api/threads/${selectedId.value}/events`, {
        event_type: eventFilters.event_type,
        payload_type: eventFilters.payload_type,
        role: eventFilters.role,
        q: eventFilters.q,
        offset: reset ? 0 : events.value.length,
        limit: eventFilters.limit,
      })
      events.value = reset ? page.items : events.value.concat(page.items)
      eventsTotal.value = page.total_matched
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      loadingEvents.value = false
    }
  }

  async function loadAllEvents() {
    if (!selectedId.value) return
    loadingEvents.value = true
    try {
      const limit = 500
      const all: SessionEvent[] = []
      let total = 0
      let offset = 0
      do {
        const page = await apiGet<EventPage>(`/api/threads/${selectedId.value}/events`, {
          event_type: eventFilters.event_type,
          payload_type: eventFilters.payload_type,
          role: eventFilters.role,
          q: eventFilters.q,
          offset,
          limit,
        })
        total = page.total_matched
        all.push(...page.items)
        offset += page.items.length
        if (page.items.length === 0) break
      } while (all.length < total)
      events.value = all
      eventsTotal.value = total
    } catch (error) {
      ElMessage.error(messageOf(error))
    } finally {
      loadingEvents.value = false
    }
  }

  function isEventExpanded(event: SessionEvent) {
    return expandedEvents.value.has(event.index)
  }

  function toggleEvent(event: SessionEvent) {
    const next = new Set(expandedEvents.value)
    if (next.has(event.index)) {
      next.delete(event.index)
    } else {
      next.add(event.index)
    }
    expandedEvents.value = next
  }

  function eventText(event: SessionEvent) {
    const text = event.display_text ?? ''
    if (isEventExpanded(event) || text.length <= 900) return text
    return `${text.slice(0, 900)}...`
  }

  async function saveEvent(index: number, raw: unknown) {
    if (!selectedId.value) return false
    try {
      await apiPost(`/api/threads/${selectedId.value}/events/${index}`, { raw })
      ElMessage.success('节点已更新')
      expandedEvents.value = new Set()
      return true
    } catch (error) {
      ElMessage.error(messageOf(error))
      return false
    }
  }

  async function deleteEvent(event: SessionEvent) {
    if (!selectedId.value) return
    const deleteAfter = ref(false)
    try {
      await ElMessageBox.confirm(
        h(DeleteEventConfirmContent, {
          eventIndex: event.index,
          onChange: (value: boolean) => {
            deleteAfter.value = value
          },
        }),
        '删除节点确认',
        {
          confirmButtonText: '删除节点',
          cancelButtonText: '取消',
          type: 'warning',
        },
      )
      await apiPost(`/api/threads/${selectedId.value}/events/${event.index}/delete`, {
        confirm: true,
        delete_after: deleteAfter.value,
      })
      ElMessage.success(deleteAfter.value ? '节点及之后条目已删除' : '节点已删除')
      expandedEvents.value = new Set()
      return true
    } catch {
      return false
    }
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    events,
    eventsTotal,
    loadingEvents,
    expandedEvents,
    eventFilters,
    loadEvents,
    loadAllEvents,
    isEventExpanded,
    toggleEvent,
    eventText,
    saveEvent,
    deleteEvent,
  }
}
