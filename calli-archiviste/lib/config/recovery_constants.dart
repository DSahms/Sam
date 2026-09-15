import 'constants.dart';

/// Recovery-journey chapter metadata for the separate NURA product.
/// Not part of the default memoir flow. Activated only when
/// CHAPTER_SET=recovery is set explicitly in .env.
class RecoveryChapterConstants {
  static const anchorQuestions = [
    'Where did I start?',
    'How did I get here?',
    'What bridge did I burn?',
  ];
  static const List<LifeChapter> chapterOrder = [
    LifeChapter.childhood,
    LifeChapter.adolescence,
    LifeChapter.family,
    LifeChapter.education,
    LifeChapter.career,
    LifeChapter.love,
    LifeChapter.parenthood,
    LifeChapter.loss,
    LifeChapter.turning,
    LifeChapter.theMoment,
    LifeChapter.theBottom,
    LifeChapter.theRooms,
    LifeChapter.lessons,
    LifeChapter.legacy,
    LifeChapter.nowURise,
  ];

  static const Map<LifeChapter, String> chapterTitles = {
    LifeChapter.childhood: 'Where It Started',
    LifeChapter.adolescence: 'Where It Often Starts',
    LifeChapter.family: 'Who Stayed, Who Left',
    LifeChapter.education: 'The Life I Was Living',
    LifeChapter.career: 'What I Built and What I Burned',
    LifeChapter.love: 'What Addiction Did to Love',
    LifeChapter.parenthood: 'Who I Did This For',
    LifeChapter.loss: 'Loss',
    LifeChapter.turning: 'Turning Points',
    LifeChapter.theMoment: 'The Moment',
    LifeChapter.theBottom: 'The Bottom',
    LifeChapter.theRooms: 'The Rooms',
    LifeChapter.lessons: 'Lessons',
    LifeChapter.legacy: 'Legacy',
    LifeChapter.nowURise: 'Now U Rise',
  };

  static const Map<LifeChapter, String> chapterSubtitles = {
    LifeChapter.childhood: 'Home, birthplace, the seeds',
    LifeChapter.adolescence: 'First pain, first escape, first use',
    LifeChapter.family: 'Who was lost, who stayed, who came back',
    LifeChapter.education: 'The life you were living before the turn',
    LifeChapter.career: 'What you built and what you burned',
    LifeChapter.love: 'What using did to the people you loved',
    LifeChapter.parenthood: 'Who you did this for — or in spite of',
    LifeChapter.loss: 'Grief, endings, who is still present',
    LifeChapter.turning: 'Before and after — the lines that divide',
    LifeChapter.theMoment: 'When you knew the old way would not work',
    LifeChapter.theBottom: 'What it actually looked like',
    LifeChapter.theRooms: 'NA, AA, who you met, what saved you',
    LifeChapter.lessons: 'What recovery is teaching you',
    LifeChapter.legacy: 'What you want remembered',
    LifeChapter.nowURise: 'Who you are becoming',
  };

  static const Map<LifeChapter, String> chapterIcons = {
    LifeChapter.childhood: '🏠',
    LifeChapter.adolescence: '🌱',
    LifeChapter.family: '👨‍👩‍👧‍👦',
    LifeChapter.education: '🛣️',
    LifeChapter.career: '🔥',
    LifeChapter.love: '💔',
    LifeChapter.parenthood: '👶',
    LifeChapter.loss: '🌙',
    LifeChapter.turning: '⚡',
    LifeChapter.theMoment: '💡',
    LifeChapter.theBottom: '⬇️',
    LifeChapter.theRooms: '🤝',
    LifeChapter.lessons: '📖',
    LifeChapter.legacy: '🌟',
    LifeChapter.nowURise: '🦋',
  };

  static const Map<LifeChapter, int> chapterSortOrder = {
    LifeChapter.childhood: 0,
    LifeChapter.adolescence: 1,
    LifeChapter.family: 2,
    LifeChapter.education: 3,
    LifeChapter.career: 4,
    LifeChapter.love: 5,
    LifeChapter.parenthood: 6,
    LifeChapter.loss: 7,
    LifeChapter.turning: 8,
    LifeChapter.theMoment: 9,
    LifeChapter.theBottom: 10,
    LifeChapter.theRooms: 11,
    LifeChapter.lessons: 12,
    LifeChapter.legacy: 13,
    LifeChapter.nowURise: 14,
  };

  static const Map<LifeChapter, List<String>> seedQuestions = {
    LifeChapter.childhood: [
      'Where did you start? Not the address — the feeling of home when you were small.',
      'What is the very first thing you remember that still feels true about who you became?',
      'Who outside your family made you feel seen when you were small?',
    ],
    LifeChapter.adolescence: [
      'When did you first learn you could change how you felt — even if the way was not good for you?',
      'Tell me about a friend from those years. What did you two do when nobody was watching?',
      'What did you believe at fifteen that you would never let go of — and did you?',
    ],
    LifeChapter.family: [
      'Start at the dinner table. Who sat where, and what was the mood nobody named out loud?',
      'Who in your family is still present in your recovery — and who are you still trying to reach?',
      'What is the unspoken rule in your family that addiction learned to exploit?',
    ],
    LifeChapter.education: [
      'How did you get here? Walk me through a normal Tuesday before things turned.',
      'What were you proud of then that nobody in recovery knows about?',
      'When did the life you were living stop matching who you thought you were?',
    ],
    LifeChapter.career: [
      'What bridge did you burn that you still think about? Be specific — one bridge, one moment.',
      'What did you build with your own hands that you are still proud of?',
      'What did burning it cost you in the body — not the story, the physical feeling?',
      'Who at work saw the real you, and what did they say?',
    ],
    LifeChapter.love: [
      'Who did you love hardest while you were still using — and what did they see that you could not?',
      'Tell me about a relationship addiction damaged. Not the summary — one specific night.',
      'Who stayed when they had every reason to leave?',
    ],
    LifeChapter.parenthood: [
      'If you have children, what do you want them to know about this chapter that you have not said yet?',
      'Who did you do this for — or in spite of — when you were at your worst?',
      'What would you do differently tomorrow for someone who depends on you?',
    ],
    LifeChapter.loss: [
      'Tell me about someone you lost who still walks into the room with you.',
      'What is the hardest goodbye you have had to say because of using?',
      'How did grief change what you needed from recovery?',
    ],
    LifeChapter.turning: [
      'If you had to pick one moment that divided your life into before and after, what was happening in the room?',
      'Tell me about a decision you almost did not make that changed everything.',
      'When did everything fall apart — and what did you do in the first hour after?',
    ],
    LifeChapter.theMoment: [
      'What was the moment you knew the old way was not going to work anymore?',
      'Who was in the room — or who was not — when that knowing landed?',
      'What did your body do when you knew? Hands, breath, where you were standing.',
    ],
    LifeChapter.theBottom: [
      'Describe the bottom without making it a movie scene — what did it actually look like?',
      'What was the last thing you told yourself before you asked for help?',
      'What would you tell someone who thinks their bottom has not arrived yet?',
    ],
    LifeChapter.theRooms: [
      'Walk me into your first meeting. What did you smell, hear, and resist?',
      'Who was the first person in the rooms who made you think you might make it?',
      'What phrase from a meeting still lives in your head on a bad night?',
    ],
    LifeChapter.lessons: [
      'What do you know now that you could not have learned without going through it?',
      'What is the best advice you got in recovery — and did you take it the first time?',
      'What would surprise the person you were on day one about who you are today?',
    ],
    LifeChapter.legacy: [
      'What do you want people to say about you when you are not in the room?',
      'What is the one story from your recovery you never want forgotten?',
      'If someone reads this book in ten years, what do you need them to understand?',
    ],
    LifeChapter.nowURise: [
      'Who are you becoming that the old version of you would not believe?',
      'What does a good ordinary day look like now — walk me through it.',
      'What are you building next that is not for using — for living?',
    ],
  };
}
