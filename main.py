from model.daylio import Daylio, init_pydantic
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


def merge(daylio1: Daylio, daylio2: Daylio) -> Daylio:
    BIG_OFFSET = 1000
    # update custom moods id
    # first_pass: make sure we don't have any duplicates id
    for mood in daylio1.custom_moods:
        mood.id_ = mood.id_ + BIG_OFFSET
    for tag in daylio1.tags:
        tag.id_ = tag.id_ + BIG_OFFSET
    for entry in daylio1.day_entries:
        entry.mood = entry.mood + BIG_OFFSET
        entry.tags = [tag + BIG_OFFSET for tag in entry.tags]

    # merge
    merged = daylio1.copy(deep=True)
    merged.custom_moods += daylio2.custom_moods
    merged.tags += daylio2.tags
    merged.day_entries += daylio2.day_entries

    # second_pass: make sure we don't have any duplicate

    ## for moods
    merged.custom_moods.sort(key=lambda x: (x.custom_name, x.icon_id))
    for prev_mood, cur_mood in pairwise(merged.custom_moods):
        if prev_mood.custom_name == cur_mood.custom_name and \
                prev_mood.icon_id == cur_mood.icon_id:
            cur_mood.id_ = -1  # mark for deletion
            for entry in merged.day_entries:
                if entry.mood == cur_mood.id_:
                    entry.mood = prev_mood.id_

    merged.custom_moods = [
        mood for mood in merged.custom_moods if mood.id_ != -1
    ]

    ## for tags
    merged.tags.sort(key=lambda x: (x.name, x.icon))
    for prev_tag, cur_tag in pairwise(merged.tags):
        if prev_tag.name == cur_tag.name and prev_tag.icon == cur_tag.icon:
            cur_tag.id_ = -1
            for entry in merged.day_entries:
                entry.tags = [
                    tag if tag != cur_tag.id_ else prev_tag.id_
                    for tag in entry.tags
                ]

    merged.tags = [tag for tag in merged.tags if tag.id_ != -1]

    # finally, sort by date and update ids
    merged.custom_moods.sort(key=lambda x: x.created_at)
    merged.tags.sort(key=lambda x: x.created_at)
    merged.day_entries.sort(key=lambda x: x.datetime_)

    # ids start at 1

    for i, mood in enumerate(merged.custom_moods):
        for entry in merged.day_entries:
            if entry.mood == mood.id_:
                entry.mood = i + 1
        mood.id_ = i + 1

    # here, we have an additional difficulty: new tag ids might collide with old ids
    # so we preprocess the tags to make sure we don't have any collisions
    for tag in merged.tags:
        tag.id_ += BIG_OFFSET
    for entry in merged.day_entries:
        entry.tags = [tag + BIG_OFFSET for tag in entry.tags]

    for i, tag in enumerate(merged.tags):
        for entry in merged.day_entries:
            entry.tags = [i + 1 if t == tag.id_ else t for t in entry.tags]

        tag.id_ = i + 1
        tag.order = i + 1

    for i, entry in enumerate(merged.day_entries):
        entry.id_ = i + 1

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
