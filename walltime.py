import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import csv
import itertools
import collections
import statistics as stats
from datetime import datetime

by_builder = {}
with open('data.csv') as f:
    for line in f:
        c = csv.reader([line], delimiter=',')
        for row in c:
            date = datetime.fromisoformat(row[1])
            by_builder.setdefault(row[2], []).append((
                date, int(row[3])/60.0/60, float(row[4])
            ))

fig, (ax, ax2) = plt.subplots(2)

for builder in by_builder.keys():
    by_builder[builder].sort()

def downsample_data(data, idx, downsampler=lambda a: stats.median(a)):
    new_data = []
    length = 8*2
    window = collections.deque(map(lambda v: v[idx], itertools.islice(data, length)), maxlen=length)
    for entry in data:
        window.append(entry[idx])
        if len(window) != 0:
            new_data.append(downsampler(window))
    return new_data


# Graph the top 5 builders, by time, where the metric used is maximum time taken
# in the last `window` commits.
window = 8*4
builder_max = []
for key in by_builder.keys():
    builder_max.append((stats.median([x[1] for x in by_builder[key][-window:]]), key))
builder_max.sort()
top_5_time = []
for max_time, key in builder_max[-5:]:
    top_5_time.append(key)

for key in by_builder.keys():
    if not key in top_5_time:
        continue
    dates_list = downsample_data(by_builder[key], 0, lambda a: a[-1])
    ax.plot(dates_list, downsample_data(by_builder[key], 1), label=key)
    ax2.plot(dates_list, downsample_data(by_builder[key], 2), label=key)

ax.set(ylabel = "Hours")
ax2.set(ylabel = "CPU usage")

for a in [ax, ax2]:
    a.legend(loc='upper left')
    minor = mdates.RRuleLocator(mdates.rrulewrapper(mdates.WEEKLY))
    a.xaxis.set_minor_locator(minor)
    a.format_xdata = mdates.DateFormatter('%Y-%m-%d')
    a.grid()

plt.show()
