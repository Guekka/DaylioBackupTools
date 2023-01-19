from model.daylio import CustomMood, Daylio, Tag, init_pydantic
import sys
from itertools import pairwise
import base64
import zipfile


def load_daylio_backup(path: str) -> Daylio:
    """ 
    Daylio exports are zip files containing a base64 encoded file.
    """
    with zipfile.ZipFile(path, 'r') as zip_ref:
        with zip_ref.open('backup.daylio', 'r') as f:
            data = f.read()
            return Daylio.parse_raw(base64.b64decode(data))


def store_daylio_backup(daylio: Daylio, path: str) -> None:
    with zipfile.ZipFile(path, 'w') as zip_ref:
        j = daylio.json(by_alias=True, exclude_unset=True, exclude_none=True)
        data = base64.b64encode(j.encode('utf-8'))
        zip_ref.writestr('backup.daylio', data)


class IdGenerator:

    def __init__(self, offset: int, start: int) -> None:
        self.offset = offset
        self.current = start

    def __call__(self) -> int:
        self.current += self.offset
        return self.current


def change_mood_id(daylio: Daylio, mood: CustomMood, new_id: int) -> None:
    for entry in daylio.day_entries:
        if entry.mood == mood.id_:
            entry.mood = new_id

    mood.id_ = new_id


def change_tag_id(daylio: Daylio, tag: Tag, new_id: int) -> None:
    for entry in daylio.day_entries:
        for i, tag_id in enumerate(entry.tags):
            if tag_id == tag.id_:
                entry.tags[i] = new_id
                break  # there can be only one

    tag.id_ = new_id


def is_duplicate_mood(mood1: CustomMood, mood2: CustomMood) -> bool:
    lhs = (mood1.custom_name.lower(), mood1.icon_id, mood1.mood_group_id)
    rhs = (mood2.custom_name.lower(), mood2.icon_id, mood2.mood_group_id)
    return lhs == rhs


def is_duplicate_tag(tag1: Tag, tag2: Tag) -> bool:
    lhs = (tag1.name.lower(), tag1.icon)
    rhs = (tag2.name.lower(), tag2.icon)
    return lhs == rhs


def merge(daylio1: Daylio, daylio2: Daylio) -> Daylio:
    BIG_OFFSET = 1000
    id_generator = IdGenerator(BIG_OFFSET, BIG_OFFSET)

    # first_pass: make sure we don't have any duplicates id
    for d in (daylio1, daylio2):
        for mood in d.custom_moods:
            change_mood_id(d, mood=mood, new_id=id_generator())

        for tag in d.tags:
            change_tag_id(d, tag=tag, new_id=id_generator())

    # merge
    merged = daylio1.copy(deep=True)
    merged.custom_moods += daylio2.custom_moods
    merged.tags += daylio2.tags
    merged.day_entries += daylio2.day_entries

    # second_pass: make sure we don't have any duplicate

    ## for moods
    merged.custom_moods.sort(key=lambda x: (x.custom_name.lower(), x.icon_id))
    for prev_mood, cur_mood in pairwise(merged.custom_moods):
        if is_duplicate_mood(prev_mood, cur_mood):
            change_mood_id(merged, mood=cur_mood, new_id=prev_mood.id_)
            cur_mood.id_ = -1  # mark for deletion

    merged.custom_moods = [
        mood for mood in merged.custom_moods if mood.id_ != -1
    ]

    ## for tags
    merged.tags.sort(key=lambda x: (x.name.lower(), x.icon))
    for prev_tag, cur_tag in pairwise(merged.tags):
        if is_duplicate_tag(prev_tag, cur_tag):
            change_tag_id(merged, tag=cur_tag, new_id=prev_tag.id_)
            cur_tag.id_ = -1  # mark for deletion

    merged.tags = [tag for tag in merged.tags if tag.id_ != -1]

    ## for entries
    merged.day_entries.sort(key=lambda x: (x.datetime_, x.year, x.month))
    for prev_entry, cur_entry in pairwise(merged.day_entries):
        # we do not want to lose any data, so they need to be exactly the same
        if prev_entry.datetime_ == cur_entry.datetime_:
            cur_entry.id_ = -1

    merged.day_entries = [
        entry for entry in merged.day_entries if entry.id_ != -1
    ]

    # finally, sort by date and update ids
    merged.custom_moods.sort(key=lambda x: (x.created_at, x.mood_group_id))
    merged.tags.sort(key=lambda x: x.created_at)
    merged.day_entries.sort(key=lambda x: (x.datetime_, x.year, x.month))

    # ids start at 1
    id_generator = IdGenerator(1, 1)
    for mood in merged.custom_moods:
        change_mood_id(merged, mood=mood, new_id=id_generator())

    id_generator = IdGenerator(1, 1)
    for i, tag in enumerate(merged.tags):
        change_tag_id(merged, tag=tag, new_id=id_generator())
        tag.order = i

    id_generator = IdGenerator(1, 1)
    for entry in merged.day_entries:
        entry.id_ = id_generator()

    # update metadata
    merged.metadata.number_of_entries = len(merged.day_entries)
    merged.metadata.number_of_photos = daylio1.metadata.number_of_photos + daylio2.metadata.number_of_photos
    merged.metadata.photos_size = daylio1.metadata.photos_size + daylio2.metadata.photos_size

    return merged


def main() -> None:
    """
    Merges two daylio json files into one.
    We assume the files have version 15, but this is not checked.
    We keep everything from the first file, and add the new entries from the second file
    """

    if len(sys.argv) not in (3, 4):
        print(
            "Usage: python main.py <main.daylio> <new.daylio> [<out.daylio>]")
        sys.exit(1)

    init_pydantic()

    old = load_daylio_backup(sys.argv[1])
    new = load_daylio_backup(sys.argv[2])

    if old.version != 15 or new.version != 15:
        print("Warning: version is not 15. This may not work.")

    out = "out.daylio"
    if len(sys.argv) == 4:
        out = sys.argv[3]

    merged = merge(old, new)
    store_daylio_backup(merged, out)


if __name__ == "__main__":
    main()
