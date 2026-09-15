/// Life chapter enum representing the 11 chapters of autobiography
enum LifeChapter {
  childhood,
  adolescence,
  family,
  education,
  career,
  love,
  parenthood,
  loss,
  turning,
  lessons,
  legacy,
  // Recovery-only chapters (ignored in memoir mode)
  theMoment,
  theBottom,
  theRooms,
  nowURise,
}

/// Constants for chapter metadata and seed questions
class ChapterConstants {
  static const Map<LifeChapter, String> chapterTitles = {
    LifeChapter.childhood: 'Childhood',
    LifeChapter.adolescence: 'Adolescence',
    LifeChapter.family: 'Family',
    LifeChapter.education: 'Education',
    LifeChapter.career: 'Career',
    LifeChapter.love: 'Love',
    LifeChapter.parenthood: 'Parenthood',
    LifeChapter.loss: 'Loss',
    LifeChapter.turning: 'Turning Points',
    LifeChapter.lessons: 'Lessons',
    LifeChapter.legacy: 'Legacy',
  };

  static const Map<LifeChapter, String> chapterSubtitles = {
    LifeChapter.childhood: 'Your earliest memories',
    LifeChapter.adolescence: 'Growing up and finding yourself',
    LifeChapter.family: 'The people who raised you',
    LifeChapter.education: 'Learning and growth',
    LifeChapter.career: 'Work and purpose',
    LifeChapter.love: 'Connections and relationships',
    LifeChapter.parenthood: 'Raising the next generation',
    LifeChapter.loss: 'Grief and change',
    LifeChapter.turning: 'Moments that changed everything',
    LifeChapter.lessons: 'What life taught you',
    LifeChapter.legacy: 'What you want to leave behind',
  };

  static const Map<LifeChapter, String> chapterIcons = {
    LifeChapter.childhood: '🏠',
    LifeChapter.adolescence: '🌱',
    LifeChapter.family: '👨‍👩‍👧‍👦',
    LifeChapter.education: '📚',
    LifeChapter.career: '💼',
    LifeChapter.love: '💕',
    LifeChapter.parenthood: '👶',
    LifeChapter.loss: '🌙',
    LifeChapter.turning: '⚡',
    LifeChapter.lessons: '💡',
    LifeChapter.legacy: '🌟',
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
    LifeChapter.lessons: 9,
    LifeChapter.legacy: 10,
  };

  /// Seed questions for each chapter (3 per chapter)
  static const Map<LifeChapter, List<String>> seedQuestions = {
    LifeChapter.childhood: [
      'What is the very first thing you can remember? Not something you were told happened — something you actually see when you close your eyes.',
      'Tell me about the house or place you lived in when you were small. If you walked through the front door right now, what would you see?',
      'Who was the first person outside your family who made you feel like you mattered?',
    ],
    LifeChapter.adolescence: [
      'When did you first realize you were becoming a different person than the kid you had been?',
      'Tell me about a friend from those years who shaped who you became. What did you two do together?',
      'What is something you believed with absolute certainty when you were fifteen?',
    ],
    LifeChapter.family: [
      'Let us start with the dinner table. What did a typical meal look like in your family? Who sat where? What was the mood?',
      'Tell me about one of your parents — not the facts of their life, but what they were like. How did they move through the world?',
      'What was the unspoken rule in your family? The thing everyone knew but nobody said out loud?',
    ],
    LifeChapter.education: [
      'Was there a teacher who changed the way you saw yourself? Tell me about them.',
      'What subject lit you up, and which one made you want to disappear?',
      'Tell me about a moment in school where you surprised yourself — for better or worse.',
    ],
    LifeChapter.career: [
      'How did you end up doing what you do? Was it a straight line or a winding road?',
      'Tell me about a day at work that you still think about. What made it stick?',
      'Who was the person who taught you the most about how to work? Not a teacher — someone in the work itself.',
    ],
    LifeChapter.love: [
      'Tell me about the first time you fell in love. Not the story of how you met — the moment you knew.',
      'What did you learn about yourself from your most important relationship?',
      'What does a good day with the person you love look like? Walk me through it.',
    ],
    LifeChapter.parenthood: [
      'Tell me about the moment you first held your child. What went through your mind?',
      'What surprised you most about becoming a parent?',
      'What is something you do as a parent that you got from your own parents, and something you deliberately do differently?',
    ],
    LifeChapter.loss: [
      'Tell me about someone you have lost who still feels present in your life.',
      'What is the hardest goodbye you have ever had to say?',
      'How did grief change you? Not just what you felt, but who you became after.',
    ],
    LifeChapter.turning: [
      'If you had to pick one moment that divided your life into before and after, what would it be?',
      'Tell me about a decision you almost did not make that changed everything.',
      'Was there a time when everything fell apart? What did you do next?',
    ],
    LifeChapter.lessons: [
      'What do you know now that you wish you had known at twenty-five?',
      'What is the best piece of advice you have ever received, and did you take it?',
      'If your younger self could see you now, what would surprise them most?',
    ],
    LifeChapter.legacy: [
      'What do you want people to say about you when you are not in the room?',
      'If you could write a letter to someone who will read it fifty years from now, what would you want them to know?',
      'What is the one story from your life that you never want forgotten?',
    ],
  };
}
